//! Playground compilation endpoints.
//!
//! **Architecture Flow:**
//! ```text
//! POST /playground/compile (fire-and-forget)
//!   ↓
//! Spawn compilation task
//!   ↓
//! tokenize() → TokenizationResult
//!   ↓
//! Publish ServerEvent::Compilation(Stage::Tokens)
//!   ↓
//! parse() → ParsingResult
//!   ↓
//! Publish ServerEvent::Compilation(Stage::Tree)
//!   ↓
//! analyze() → SemanticResult
//!   ↓
//! Publish ServerEvent::Compilation(Stage::Sir)
//!   ↓
//! codegen() → Assembly text
//!   ↓
//! Publish ServerEvent::Compilation(Stage::Asm)
//!   ↓
//! RPC WebSocket broadcasts to clients
//! ```

use crate::state::ServerState;

use codelord_protocol::compilation::{CompilationEvent, CompileRequest, Stage};
use codelord_protocol::event::ServerEvent;

use zo_analyzer::Analyzer;
use zo_codegen::codegen::Codegen;
use zo_codegen_backend::Target;
use zo_parser::Parser;
use zo_runtime_web::HtmlRenderer;
use zo_sir::{BinOp, Insn, Sir, UnOp};
use zo_token::TokenBuffer;
use zo_tokenizer::{TokenizationResult, Tokenizer};
use zo_tree::{NodeValue, Tree};
use zo_ui_protocol::UiCommand;

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, Router, routing};

use std::sync::Arc;

pub fn router(state: Arc<ServerState>) -> Router {
  Router::new()
    .route("/compile", routing::post(compile))
    .with_state(state)
}

/// POST /playground/compile
///
/// Accepts source code, compiles it in background, and streams
/// compilation events via WebSocket.
async fn compile(
  State(state): State<Arc<ServerState>>,
  Json(request): Json<CompileRequest>,
) -> StatusCode {
  tracing::info!("[Playground] Compile request received");

  let target_stage = request.stage;

  // Spawn compilation in background task.
  tokio::spawn(async move {
    // Send started event.
    let _ = state
      .event_bus
      .send(ServerEvent::Compilation(CompilationEvent::Started));

    // Stage: Tokens (always runs).
    let start = std::time::Instant::now();
    let mut tokenization = tokenize(&request.source);
    let elapsed_time = start.elapsed().as_nanos() as f64 / 1_000_000.0;
    let tokens_json = serialize_tokens(&tokenization.tokens, &request.source);

    let _ =
      state
        .event_bus
        .send(ServerEvent::Compilation(CompilationEvent::Stage {
          stage: Stage::Tokens,
          data: tokens_json,
          elapsed_time,
        }));

    if target_stage == Stage::Tokens {
      let _ = state.event_bus.send(ServerEvent::Compilation(
        CompilationEvent::Done { success: true },
      ));
      tracing::info!("[Playground] Compilation completed (Tokens)");
      return;
    }

    // Stage: Tree.
    let start = std::time::Instant::now();
    let parsing = parse(&tokenization, &request.source);
    let elapsed_time = start.elapsed().as_nanos() as f64 / 1_000_000.0;
    let tree_json = serialize_tree(&parsing.tree, &request.source);

    let _ =
      state
        .event_bus
        .send(ServerEvent::Compilation(CompilationEvent::Stage {
          stage: Stage::Tree,
          data: tree_json,
          elapsed_time,
        }));

    if target_stage == Stage::Tree {
      let _ = state.event_bus.send(ServerEvent::Compilation(
        CompilationEvent::Done { success: true },
      ));
      tracing::info!("[Playground] Compilation completed (Tree)");
      return;
    }

    // Stage: SIR.
    let start = std::time::Instant::now();
    let semantic = analyze(&parsing.tree, &mut tokenization);
    let elapsed_time = start.elapsed().as_nanos() as f64 / 1_000_000.0;
    let sir_json = serialize_sir(&semantic.sir, &tokenization.interner);

    let _ =
      state
        .event_bus
        .send(ServerEvent::Compilation(CompilationEvent::Stage {
          stage: Stage::Sir,
          data: sir_json,
          elapsed_time,
        }));

    if target_stage == Stage::Sir {
      let _ = state.event_bus.send(ServerEvent::Compilation(
        CompilationEvent::Done { success: true },
      ));
      tracing::info!("[Playground] Compilation completed (SIR)");
      return;
    }

    // Stage: Asm (Programming mode).
    if target_stage == Stage::Asm {
      let start = std::time::Instant::now();
      let asm = codegen(&semantic.sir, &tokenization.interner);
      let elapsed_time = start.elapsed().as_nanos() as f64 / 1_000_000.0;

      let _ = state.event_bus.send(ServerEvent::Compilation(
        CompilationEvent::Stage {
          stage: Stage::Asm,
          data: asm,
          elapsed_time,
        },
      ));

      let _ = state.event_bus.send(ServerEvent::Compilation(
        CompilationEvent::Done { success: true },
      ));

      tracing::info!("[Playground] Compilation completed (Asm)");
      return;
    }

    // Stage: Ui (Templating mode).
    let start = std::time::Instant::now();
    let (html, ui_json) = render_ui(&semantic.sir);
    let elapsed_time = start.elapsed().as_nanos() as f64 / 1_000_000.0;

    // Store HTML in server state for /preview endpoint.
    {
      let mut preview_html = state.preview_html.lock().await;
      *preview_html = html;
    }

    let _ =
      state
        .event_bus
        .send(ServerEvent::Compilation(CompilationEvent::Stage {
          stage: Stage::Ui,
          data: ui_json,
          elapsed_time,
        }));

    // Send done event.
    let _ =
      state
        .event_bus
        .send(ServerEvent::Compilation(CompilationEvent::Done {
          success: true,
        }));

    tracing::info!("[Playground] Compilation completed (Ui)");
  });

  StatusCode::ACCEPTED
}

/// Tokenize source code.
fn tokenize(source: &str) -> TokenizationResult {
  Tokenizer::new(source).tokenize()
}

/// A token with its lexeme for serialization.
#[derive(serde::Serialize)]
struct Token<'a> {
  kind: zo_token::Token,
  lexeme: &'a str,
  span: (usize, usize),
}

/// Tokens output for serialization.
#[derive(serde::Serialize)]
struct TokensOutput<'a> {
  tokens: Vec<Token<'a>>,
}

/// Serialize tokens to JSON with lexemes.
fn serialize_tokens(tokens: &TokenBuffer, source: &str) -> String {
  let output = TokensOutput {
    tokens: tokens
      .kinds
      .iter()
      .enumerate()
      .map(|(i, &kind)| {
        let start = tokens.starts[i] as usize;
        let len = tokens.lengths[i] as usize;
        let lexeme = &source[start..start + len];

        Token {
          kind,
          lexeme,
          span: (start, start + len),
        }
      })
      .collect(),
  };

  sonic_rs::to_string(&output).unwrap_or_default()
}

/// Parse tokenized source code.
fn parse<'a>(
  tokenization: &'a TokenizationResult,
  source: &'a str,
) -> zo_parser::ParsingResult {
  Parser::new(tokenization, source).parse()
}

/// A tree node for serialization.
#[derive(serde::Serialize)]
struct Node<'a> {
  token: zo_token::Token,
  lexeme: &'a str,
  span: (usize, usize),
  child_start: u16,
  child_count: u16,
  value: Option<SerializedValue>,
}

/// A serializable node value.
#[derive(serde::Serialize)]
enum SerializedValue {
  Symbol(u32),
  Literal(u32),
  TextRange(u32, u16),
}

/// Tree output for serialization.
#[derive(serde::Serialize)]
struct TreeOutput<'a> {
  nodes: Vec<Node<'a>>,
}

/// Serialize tree to JSON with lexemes.
fn serialize_tree(tree: &Tree, source: &str) -> String {
  let output = TreeOutput {
    nodes: tree
      .nodes
      .iter()
      .enumerate()
      .map(|(i, header)| {
        let span = tree.spans[i];
        let start = span.start as usize;
        let len = span.len as usize;
        let lexeme = if start + len <= source.len() {
          &source[start..start + len]
        } else {
          ""
        };

        let value = tree.value(i as u32).map(|v| match v {
          NodeValue::Symbol(sym) => SerializedValue::Symbol(sym.0),
          NodeValue::Literal(idx) => SerializedValue::Literal(idx),
          NodeValue::TextRange(s, l) => SerializedValue::TextRange(s, l),
        });

        Node {
          token: header.token,
          lexeme,
          span: (start, start + len),
          child_start: header.child_start,
          child_count: header.child_count,
          value,
        }
      })
      .collect(),
  };

  sonic_rs::to_string(&output).unwrap_or_default()
}

/// Analyze parse tree to produce SIR.
fn analyze(
  tree: &Tree,
  tokenization: &mut TokenizationResult,
) -> zo_analyzer::SemanticResult {
  Analyzer::new(tree, &mut tokenization.interner, &tokenization.literals)
    .analyze()
}

/// A SIR instruction for serialization.
#[derive(serde::Serialize)]
struct SirInsn {
  kind: String,
  details: String,
}

/// SIR output for serialization.
#[derive(serde::Serialize)]
struct SirOutput {
  instructions: Vec<SirInsn>,
}

/// Serialize SIR to JSON.
fn serialize_sir(sir: &Sir, interner: &zo_interner::Interner) -> String {
  let output = SirOutput {
    instructions: sir
      .instructions
      .iter()
      .map(|insn| {
        let (kind, details) = match insn {
          Insn::ConstInt { value, ty_id, .. } => {
            ("ConstInt".into(), format!("{value} : ty{}", ty_id.0))
          }
          Insn::ConstFloat { value, ty_id, .. } => {
            ("ConstFloat".into(), format!("{value} : ty{}", ty_id.0))
          }
          Insn::ConstBool { value, ty_id, .. } => {
            ("ConstBool".into(), format!("{value} : ty{}", ty_id.0))
          }
          Insn::ConstString { symbol, ty_id, .. } => {
            let s = interner.get(*symbol);
            ("ConstString".into(), format!("\"{s}\" : ty{}", ty_id.0))
          }
          Insn::VarDef {
            name,
            ty_id,
            init,
            mutability,
            ..
          } => {
            let n = interner.get(*name);
            let init_str =
              init.map(|v| format!(" = v{}", v.0)).unwrap_or_default();
            (
              "VarDef".into(),
              format!("{mutability:?} {n} : ty{}{init_str}", ty_id.0),
            )
          }
          Insn::Store { name, value, ty_id } => {
            let n = interner.get(*name);
            (
              "Store".into(),
              format!("{n} = v{} : ty{}", value.0, ty_id.0),
            )
          }
          Insn::FunDef {
            name,
            params,
            return_ty,
            body_start,
            ..
          } => {
            let n = interner.get(*name);
            let params_str: Vec<_> = params
              .iter()
              .map(|(p, t)| {
                let name = interner.get(*p);
                format!("{name}: ty{}", t.0)
              })
              .collect();
            (
              "FunDef".into(),
              format!(
                "{n}({}) -> ty{} @ {body_start}",
                params_str.join(", "),
                return_ty.0
              ),
            )
          }
          Insn::Return { value, ty_id } => {
            let v = value.map(|v| format!("v{}", v.0)).unwrap_or("void".into());
            ("Return".into(), format!("{v} : ty{}", ty_id.0))
          }
          Insn::Call {
            name, args, ty_id, ..
          } => {
            let n = interner.get(*name);
            let args_str: Vec<_> =
              args.iter().map(|a| format!("v{}", a.0)).collect();
            (
              "Call".into(),
              format!("{n}({}) : ty{}", args_str.join(", "), ty_id.0),
            )
          }
          Insn::Load { dst, src, ty_id } => (
            "Load".into(),
            format!("v{} = param[{src:?}] : ty{}", dst.0, ty_id.0),
          ),
          Insn::BinOp {
            dst,
            op,
            lhs,
            rhs,
            ty_id,
          } => {
            let op_str = match op {
              BinOp::Add => "+",
              BinOp::Sub => "-",
              BinOp::Mul => "*",
              BinOp::Div => "/",
              BinOp::Rem => "%",
              BinOp::Eq => "==",
              BinOp::Neq => "!=",
              BinOp::Lt => "<",
              BinOp::Lte => "<=",
              BinOp::Gt => ">",
              BinOp::Gte => ">=",
              BinOp::And => "&&",
              BinOp::Or => "||",
              BinOp::BitAnd => "&",
              BinOp::BitOr => "|",
              BinOp::BitXor => "^",
              BinOp::Shl => "<<",
              BinOp::Shr => ">>",
              BinOp::Concat => "++",
            };
            (
              "BinOp".into(),
              format!(
                "v{} = v{} {op_str} v{} : ty{}",
                dst.0, lhs.0, rhs.0, ty_id.0
              ),
            )
          }
          Insn::UnOp { op, rhs, ty_id, .. } => {
            let op_str = match op {
              UnOp::Neg => "-",
              UnOp::Not => "!",
              UnOp::Ref => "&",
              UnOp::Deref => "*",
              UnOp::BitNot => "~",
            };
            ("UnOp".into(), format!("{op_str}v{} : ty{}", rhs.0, ty_id.0))
          }
          Insn::Directive { name, value, ty_id } => {
            let n = interner.get(*name);
            (
              "Directive".into(),
              format!("#{n} v{} : ty{}", value.0, ty_id.0),
            )
          }
          Insn::Template {
            id,
            name,
            ty_id,
            commands,
          } => {
            let tag = name.map(|s| interner.get(s)).unwrap_or("<>");
            (
              "Template".into(),
              format!(
                "v{} = <{tag}> ({} cmds) : ty{}",
                id.0,
                commands.len(),
                ty_id.0
              ),
            )
          }
          other => ("Unknown".into(), format!("{other:?}")),
        };
        SirInsn { kind, details }
      })
      .collect(),
  };

  sonic_rs::to_string(&output).unwrap_or_default()
}

/// Generate ARM64 assembly text from SIR.
fn codegen(sir: &Sir, interner: &zo_interner::Interner) -> String {
  let codegen = Codegen::new(Target::Arm64AppleDarwin);
  codegen.generate_asm(interner, sir)
}

/// Extract UiCommands from SIR and render to HTML.
/// Returns (html_string, ui_commands_json).
fn render_ui(sir: &Sir) -> (String, String) {
  // Extract UiCommands from Template instructions.
  let mut ui_commands: Vec<UiCommand> = Vec::new();

  for insn in &sir.instructions {
    if let Insn::Template { commands, .. } = insn {
      ui_commands.extend_from_slice(commands);
    }
  }

  // Render to HTML using HtmlRenderer.
  let mut renderer = HtmlRenderer::new();
  let html = renderer.render_to_html(&ui_commands);

  // Serialize commands as JSON for native rendering and display.
  let ui_json = sonic_rs::to_string(&UiOutput {
    commands: ui_commands,
  })
  .unwrap_or_default();

  (html, ui_json)
}

/// UI output for serialization.
#[derive(serde::Serialize)]
struct UiOutput {
  commands: Vec<UiCommand>,
}
