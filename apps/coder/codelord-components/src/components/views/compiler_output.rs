use codelord_core::ecs::world::World;
use codelord_core::navigation::PlaygroundMode;
use codelord_core::navigation::resources::StagebarResource;
use codelord_core::playground::{PlaygroundHoveredSpan, PlaygroundOutput};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// Show compiler output view.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let (selected_stage, mode) = world
    .get_resource::<StagebarResource>()
    .map(|s| (s.selected, s.mode))
    .unwrap_or((0, PlaygroundMode::Programming));

  let output = world.get_resource::<PlaygroundOutput>();

  let content = output.and_then(|o| match selected_stage {
    0 => o.compilation.tokens.to_owned(),
    1 => o.compilation.tree.to_owned(),
    2 => o.compilation.sir.to_owned(),
    3 => match mode {
      PlaygroundMode::Programming => o.compilation.asm.to_owned(),
      PlaygroundMode::Templating => o.compilation.ui.to_owned(),
    },
    _ => None,
  });

  match content {
    Some(data) => match (selected_stage, mode) {
      (0, _) => show_tokens(ui, world, &data),
      (1, _) => show_tree(ui, world, &data),
      (2, _) => show_sir(ui, world, &data),
      (3, PlaygroundMode::Programming) => show_asm(ui, &data),
      (3, PlaygroundMode::Templating) => show_ui(ui, &data),
      _ => show_raw(ui, &data),
    },
    None => show_empty_state(ui, selected_stage, mode),
  }
}

/// Show tokens output.
fn show_tokens(ui: &mut egui::Ui, world: &mut World, data: &str) {
  if let Ok(output) = sonic_rs::from_str::<TokensOutput>(data) {
    render_tokens(ui, world, &output.tokens);
  } else {
    show_raw(ui, data);
  }
}

/// Show empty state message.
fn show_empty_state(ui: &mut egui::Ui, stage: usize, mode: PlaygroundMode) {
  let stage_name = match (stage, mode) {
    (0, _) => "Tokens",
    (1, _) => "Tree",
    (2, _) => "SiR",
    (3, PlaygroundMode::Programming) => "Asm",
    (3, PlaygroundMode::Templating) => "Ui",
    _ => "Output",
  };

  ui.centered_and_justified(|ui| {
    ui.add_space(8.0);
    ui.label(
      egui::RichText::new(format!(
        "No {stage_name} to display. Click Run to compile."
      ))
      .color(ui.visuals().weak_text_color())
      .size(12.0),
    );
  });
}

/// Token with lexeme (matches server output).
#[derive(serde::Deserialize)]
struct TokenBuffer {
  kind: String,
  lexeme: String,
  span: (usize, usize),
}

/// Tokens output structure.
#[derive(serde::Deserialize)]
struct TokensOutput {
  tokens: Vec<TokenBuffer>,
}

/// Render tokens in a table view.
fn render_tokens(ui: &mut egui::Ui, world: &mut World, tokens: &[TokenBuffer]) {
  if tokens.is_empty() {
    ui.centered_and_justified(|ui| {
      ui.label(
        egui::RichText::new("No tokens")
          .color(ui.visuals().weak_text_color())
          .size(12.0),
      );
    });
    return;
  }

  let available_height = ui.available_height();

  // Track hovered span during this frame.
  let mut hovered_span: Option<(usize, usize)> = None;

  ui.horizontal(|ui| {
    ui.add_space(8.0);

    ui.vertical(|ui| {
      ui.add_space(16.0);

      TableBuilder::new(ui)
        .striped(false)
        .sense(egui::Sense::hover())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(100.0)) // KIND
        .column(Column::remainder().at_least(150.0)) // LEXEME
        .column(Column::exact(100.0)) // SPAN
        .min_scrolled_height(available_height)
        .max_scroll_height(available_height)
        .header(20.0, |mut header| {
          header.col(|ui| {
            ui.label(
              egui::RichText::new("TYPE")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("LEXEME")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("SPAN")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
        })
        .body(|mut body| {
          for token in tokens.iter().filter(|t| t.kind != "Eof") {
            let kind_color = token_color(&token.kind);
            let span_text = format!("({}..{})", token.span.0, token.span.1);
            let token_span = token.span;

            body.row(18.0, |mut row| {
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(&token.kind)
                    .monospace()
                    .size(11.0)
                    .color(kind_color),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(&token.lexeme)
                    .monospace()
                    .size(11.0)
                    .color(kind_color),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(span_text)
                    .monospace()
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
                );
              });

              // Check if row is hovered (must be after all cols).
              if row.response().hovered() {
                hovered_span = Some(token_span);
              }
            });
          }
        });
    });
  });

  // Update the hovered span resource.
  if let Some(mut resource) = world.get_resource_mut::<PlaygroundHoveredSpan>()
  {
    resource.span = hovered_span;
  }
}

/// Get color for token kind.
fn token_color(kind: &str) -> egui::Color32 {
  match kind {
    // Keywords.
    "Fun" | "Fn" | "If" | "Else" | "While" | "For" | "Loop" | "Return"
    | "Break" | "Continue" | "Match" | "When" | "Pub" | "Mut" | "Imu"
    | "Val" | "Type" | "Struct" | "Enum" | "Pack" | "Load" => {
      egui::Color32::from_rgb(198, 120, 221)
    }

    // Types.
    "IntType" | "S8Type" | "S16Type" | "S32Type" | "S64Type" | "UintType"
    | "U8Type" | "U16Type" | "U32Type" | "U64Type" | "FloatType"
    | "F32Type" | "F64Type" | "BoolType" | "CharType" | "StrType"
    | "BytesType" => egui::Color32::from_rgb(86, 182, 194),

    // Literals.
    "Int" | "Float" | "String" | "RawString" | "Char" | "Bytes" | "True"
    | "False" => egui::Color32::from_rgb(209, 154, 102),

    // Identifiers.
    "Ident" => egui::Color32::WHITE,

    // Operators and punctuation.
    _ => egui::Color32::GRAY,
  }
}

/// Show tree output.
fn show_tree(ui: &mut egui::Ui, world: &mut World, data: &str) {
  if let Ok(output) = sonic_rs::from_str::<TreeOutput>(data) {
    render_tree(ui, world, &output.nodes);
  } else {
    show_raw(ui, data);
  }
}

/// Show raw data as fallback.
fn show_raw(ui: &mut egui::Ui, data: &str) {
  ui.horizontal(|ui| {
    ui.add_space(8.0);
    ui.label(
      egui::RichText::new(data)
        .monospace()
        .size(12.0)
        .color(ui.visuals().text_color()),
    );
  });
}

/// Tree node (matches server output).
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct TreeNode {
  token: String,
  lexeme: String,
  span: (usize, usize),
  child_start: u16,
  child_count: u16,
  value: Option<NodeValue>,
}

/// Node value variants.
#[derive(serde::Deserialize)]
#[allow(dead_code)]
enum NodeValue {
  Symbol(u32),
  Literal(u32),
  TextRange(u32, u16),
}

/// Tree output structure.
#[derive(serde::Deserialize)]
struct TreeOutput {
  nodes: Vec<TreeNode>,
}

/// Render tree nodes in a table view.
fn render_tree(ui: &mut egui::Ui, world: &mut World, nodes: &[TreeNode]) {
  if nodes.is_empty() {
    ui.centered_and_justified(|ui| {
      ui.label(
        egui::RichText::new("No nodes")
          .color(ui.visuals().weak_text_color())
          .size(12.0),
      );
    });
    return;
  }

  let available_height = ui.available_height();

  // Track hovered span during this frame.
  let mut hovered_span: Option<(usize, usize)> = None;

  ui.horizontal(|ui| {
    ui.add_space(8.0);

    ui.vertical(|ui| {
      ui.add_space(16.0);

      TableBuilder::new(ui)
        .striped(false)
        .sense(egui::Sense::hover())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(40.0)) // INDEX
        .column(Column::exact(120.0)) // TOKEN
        .column(Column::remainder().at_least(100.0)) // LEXEME
        .column(Column::exact(80.0)) // SPAN
        .column(Column::exact(80.0)) // CHILDREN
        .min_scrolled_height(available_height)
        .max_scroll_height(available_height)
        .header(20.0, |mut header| {
          header.col(|ui| {
            ui.label(
              egui::RichText::new("#")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("TOKEN")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("LEXEME")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("SPAN")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("CHiLDREN")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
        })
        .body(|mut body| {
          for (idx, node) in
            nodes.iter().enumerate().filter(|(_, n)| n.token != "Eof")
          {
            let kind_color = token_color(&node.token);
            let span_text = format!("{}..{}", node.span.0, node.span.1);
            let children_text = if node.child_count > 0 {
              format!(
                "{}..{}",
                node.child_start,
                node.child_start + node.child_count
              )
            } else {
              "—".to_string()
            };
            let node_span = node.span;

            body.row(18.0, |mut row| {
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(format!("{idx}"))
                    .monospace()
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(&node.token)
                    .monospace()
                    .size(11.0)
                    .color(kind_color),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(&node.lexeme)
                    .monospace()
                    .size(11.0)
                    .color(kind_color),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(span_text)
                    .monospace()
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                let color = if node.child_count > 0 {
                  egui::Color32::from_rgb(86, 182, 194)
                } else {
                  ui.visuals().weak_text_color()
                };
                ui.label(
                  egui::RichText::new(children_text)
                    .monospace()
                    .size(11.0)
                    .color(color),
                );
              });

              // Check if row is hovered (must be after all cols).
              if row.response().hovered() {
                hovered_span = Some(node_span);
              }
            });
          }
        });
    });
  });

  // Update the hovered span resource.
  if let Some(mut resource) = world.get_resource_mut::<PlaygroundHoveredSpan>()
  {
    resource.span = hovered_span;
  }
}

/// Show SIR output.
fn show_sir(ui: &mut egui::Ui, _world: &mut World, data: &str) {
  if let Ok(output) = sonic_rs::from_str::<SirOutput>(data) {
    render_sir(ui, &output.instructions);
  } else {
    show_raw(ui, data);
  }
}

/// SIR instruction (matches server output).
#[derive(serde::Deserialize)]
struct SirInsn {
  kind: String,
  details: String,
}

/// SIR output structure.
#[derive(serde::Deserialize)]
struct SirOutput {
  instructions: Vec<SirInsn>,
}

/// Render SIR instructions in a table view.
fn render_sir(ui: &mut egui::Ui, instructions: &[SirInsn]) {
  if instructions.is_empty() {
    ui.centered_and_justified(|ui| {
      ui.label(
        egui::RichText::new("No instructions")
          .color(ui.visuals().weak_text_color())
          .size(12.0),
      );
    });
    return;
  }

  let available_height = ui.available_height();

  ui.horizontal(|ui| {
    ui.add_space(8.0);

    ui.vertical(|ui| {
      ui.add_space(16.0);

      TableBuilder::new(ui)
        .striped(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(40.0)) // INDEX
        .column(Column::exact(100.0)) // KIND
        .column(Column::remainder().at_least(200.0)) // DETAILS
        .min_scrolled_height(available_height)
        .max_scroll_height(available_height)
        .header(20.0, |mut header| {
          header.col(|ui| {
            ui.label(
              egui::RichText::new("#")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("TYPE")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
          header.col(|ui| {
            ui.label(
              egui::RichText::new("DETAiLS")
                .strong()
                .color(egui::Color32::WHITE),
            );
          });
        })
        .body(|mut body| {
          for (idx, insn) in instructions.iter().enumerate() {
            let kind_color = sir_color(&insn.kind);

            body.row(18.0, |mut row| {
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(format!("{idx}"))
                    .monospace()
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(&insn.kind)
                    .monospace()
                    .size(11.0)
                    .color(kind_color),
                );
              });
              row.col(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(
                  egui::RichText::new(&insn.details)
                    .monospace()
                    .size(11.0)
                    .color(ui.visuals().text_color()),
                );
              });
            });
          }
        });
    });
  });
}

/// Get color for SIR instruction kind.
fn sir_color(kind: &str) -> egui::Color32 {
  match kind {
    // Constants
    "ConstInt" | "ConstFloat" | "ConstBool" | "ConstString" => {
      egui::Color32::from_rgb(209, 154, 102) // Orange - literals
    }
    // Definitions
    "VarDef" | "FunDef" => {
      egui::Color32::from_rgb(198, 120, 221) // Purple - declarations
    }
    // Control flow
    "Return" | "Call" => {
      egui::Color32::from_rgb(86, 182, 194) // Cyan - control
    }
    // Operations
    "BinOp" | "UnOp" => {
      egui::Color32::from_rgb(152, 195, 121) // Green - operations
    }
    // Memory
    "Load" | "Store" => {
      egui::Color32::from_rgb(224, 108, 117) // Red - memory
    }
    // Special
    "Directive" | "Template" => {
      egui::Color32::from_rgb(229, 192, 123) // Yellow - special
    }
    _ => egui::Color32::GRAY,
  }
}

/// Show assembly output with syntax highlighting.
fn show_asm(ui: &mut egui::Ui, data: &str) {
  let available_size = ui.available_size();

  ui.horizontal(|ui| {
    ui.add_space(8.0);

    ui.vertical(|ui| {
      ui.add_space(16.0);

      egui::ScrollArea::both()
        .min_scrolled_height(available_size.y)
        .max_height(available_size.y)
        .min_scrolled_width(available_size.x - 16.0)
        .max_width(available_size.x - 16.0)
        .show(ui, |ui| {
          for line in data.lines() {
            let trimmed = line.trim();

            // Color based on line content.
            let color = if trimmed.starts_with(';') || trimmed.starts_with('#')
            {
              // Comments and directives.
              egui::Color32::from_rgb(106, 115, 125)
            } else if trimmed.starts_with('.') {
              // Assembler directives.
              egui::Color32::from_rgb(198, 120, 221)
            } else if trimmed.ends_with(':') {
              // Labels.
              egui::Color32::from_rgb(86, 182, 194)
            } else if trimmed.starts_with("mov")
              || trimmed.starts_with("add")
              || trimmed.starts_with("sub")
              || trimmed.starts_with("mul")
              || trimmed.starts_with("div")
              || trimmed.starts_with("ret")
              || trimmed.starts_with("bl")
              || trimmed.starts_with("svc")
              || trimmed.starts_with("adr")
              || trimmed.starts_with("ldr")
              || trimmed.starts_with("str")
              || trimmed.starts_with("cmp")
            {
              // Instructions.
              egui::Color32::from_rgb(152, 195, 121)
            } else {
              ui.visuals().text_color()
            };

            ui.label(
              egui::RichText::new(line)
                .monospace()
                .size(11.0)
                .color(color),
            );
          }
        });
    });
  });
}

/// Show UI commands output.
fn show_ui(ui: &mut egui::Ui, data: &str) {
  #[derive(serde::Deserialize)]
  struct UiOutput {
    commands: Vec<String>,
    count: usize,
  }

  let available_size = ui.available_size();

  ui.horizontal(|ui| {
    ui.add_space(8.0);

    ui.vertical(|ui| {
      ui.add_space(16.0);

      if let Ok(output) = sonic_rs::from_str::<UiOutput>(data) {
        // Header with command count.
        ui.label(
          egui::RichText::new(format!("{} Ui Commands", output.count))
            .strong()
            .size(12.0),
        );
        ui.add_space(8.0);

        egui::ScrollArea::both()
          .min_scrolled_height(available_size.y - 40.0)
          .max_height(available_size.y - 40.0)
          .min_scrolled_width(available_size.x - 16.0)
          .max_width(available_size.x - 16.0)
          .show(ui, |ui| {
            for (i, cmd) in output.commands.iter().enumerate() {
              // Color based on command type.
              let color = if cmd.starts_with("BeginContainer") {
                egui::Color32::from_rgb(86, 182, 194) // Cyan
              } else if cmd.starts_with("EndContainer") {
                egui::Color32::from_rgb(106, 115, 125) // Gray
              } else if cmd.starts_with("Text") {
                egui::Color32::from_rgb(152, 195, 121) // Green
              } else if cmd.starts_with("Button") {
                egui::Color32::from_rgb(198, 120, 221) // Purple
              } else if cmd.starts_with("TextInput") {
                egui::Color32::from_rgb(229, 192, 123) // Yellow
              } else if cmd.starts_with("Image") {
                egui::Color32::from_rgb(224, 108, 117) // Red
              } else {
                ui.visuals().text_color()
              };

              ui.horizontal(|ui| {
                ui.label(
                  egui::RichText::new(format!("{i:3}"))
                    .monospace()
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
                );
                ui.label(
                  egui::RichText::new(cmd).monospace().size(11.0).color(color),
                );
              });
            }
          });
      } else {
        // Fallback to raw display.
        egui::ScrollArea::both()
          .min_scrolled_height(available_size.y)
          .max_height(available_size.y)
          .show(ui, |ui| {
            ui.label(
              egui::RichText::new(data)
                .monospace()
                .size(11.0)
                .color(ui.visuals().text_color()),
            );
          });
      }
    });
  });
}
