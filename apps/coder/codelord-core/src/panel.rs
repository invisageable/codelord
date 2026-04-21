pub mod components;
pub mod resources;
pub mod systems;

/// Insert panel resources + message queue.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;
  use crate::panel::resources::{
    BottomPanelResource, LeftPanelResource, PanelCommand, RightPanelResource,
  };

  world.insert_resource(LeftPanelResource::default());
  world.insert_resource(RightPanelResource::default());
  world.insert_resource(BottomPanelResource::default());
  world.init_resource::<Messages<PanelCommand>>();
}

/// Register panel systems: preview toggles, SQLite/XLS/SVG/search panels.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::panel_command_system,
    systems::toggle_html_preview_system,
    systems::update_html_preview_on_tab_change,
    systems::toggle_markdown_preview_system,
    systems::update_markdown_preview_on_change,
    systems::update_markdown_preview_on_tab_change,
    systems::toggle_csv_preview_system,
    systems::update_csv_preview_on_change,
    systems::update_csv_preview_on_tab_change,
    systems::open_pdf_preview_system,
    systems::close_pdf_preview_system,
    systems::update_pdf_preview_on_tab_change,
  ));
  schedule.add_systems((
    systems::toggle_sqlite_preview_system,
    systems::update_sqlite_preview_on_tab_change,
    systems::select_sqlite_table_system,
    systems::change_sqlite_page_system,
    systems::execute_sqlite_sql_system,
    systems::export_sqlite_data_system,
  ));
  schedule.add_systems((
    systems::select_xls_sheet_system,
    systems::change_xls_page_system,
  ));
  schedule.add_systems((
    systems::svg_zoom_in_system,
    systems::svg_zoom_out_system,
    systems::svg_zoom_reset_system,
  ));
  schedule.add_systems((
    systems::toggle_search_system,
    systems::hcodelord_search_system,
    systems::update_search_query_system,
    systems::toggle_search_option_system,
    systems::find_next_system,
    systems::find_previous_system,
    systems::execute_search_system,
  ));
}
