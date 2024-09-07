mod edit;
mod outliner;
mod settings;
mod table;
mod viewport;

use super::util::get_egui_ctx;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_pkv::PkvStore;
use edit::show_edit_tab;
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use outliner::show_outliner_tab;
use serde::{Deserialize, Serialize};
use settings::show_settings_tab;
use strum_macros::{Display, EnumIter};
use table::show_table_tab;
use viewport::show_viewport_tab;

pub fn docktree_plugin(app: &mut App) {
    app.add_systems(Startup, setup_docktree);
}

fn setup_docktree(mut commands: Commands, mut pkv: ResMut<PkvStore>) {
    // get the docktree if it exists, if not, set it to default
    let tree = match pkv.get::<DockTree>("tree") {
        Ok(tree) => tree,
        Err(_) => {
            pkv.set("tree", &DockTree::default()).unwrap();
            DockTree::default()
        }
    };
    commands.insert_resource(tree);
}

#[derive(Deref, DerefMut, Resource, Serialize, Deserialize, Clone)]
pub struct DockTree(DockState<Tab>);
impl Default for DockTree {
    fn default() -> Self {
        let mut tree = DockState::new(vec![Tab::Viewport, Tab::Table]);
        let tree_main_surface = tree.main_surface_mut();

        let [_, right] = tree_main_surface.split_right(NodeIndex::root(), 0.8, vec![Tab::Outliner]);

        tree_main_surface.split_below(right, 0.5, vec![Tab::Edit]);

        Self(tree)
    }
}

#[derive(Display, PartialEq, EnumIter, Serialize, Deserialize, Clone, Copy)]
pub enum Tab {
    Viewport,
    Outliner,
    Edit,
    Table,
    Settings,
}

// this tells egui how to render each tab

pub struct TabViewer<'a>(&'a mut World);
impl egui_dock::TabViewer for TabViewer<'_> {
    // each tab will be distinguished by an enum which can be converted to a string using strum
    type Tab = Tab;
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // we can do different things inside the tab depending on its name
        match tab {
            Tab::Viewport => show_viewport_tab(ui, self.0),
            Tab::Outliner => show_outliner_tab(ui, self.0),
            Tab::Edit => show_edit_tab(ui, self.0),
            Tab::Table => show_table_tab(ui, self.0),
            Tab::Settings => show_settings_tab(ui, self.0),
        };
    }
    // show the title of the tab - the 'Tab' type already stores its title anyway
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }
}

pub fn show_dock_area(world: &mut World) {
    let ctx = &get_egui_ctx(world);

    let style = Style::from_egui(ctx.style().as_ref());

    world.resource_scope(|world, mut tree: Mut<DockTree>| {
        // show the actual dock area
        DockArea::new(&mut tree).style(style).show(ctx, &mut TabViewer(world));
    });
}
