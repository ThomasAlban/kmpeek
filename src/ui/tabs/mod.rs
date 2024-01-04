mod settings;
mod view;
mod viewport;

pub use settings::*;
pub use view::*;
pub use viewport::*;

use super::{
    app_state::AppSettings,
    tabs::{show_settings_tab, show_viewport_tab, ViewportParams},
    tabs::{show_view_tab, SettingsParams, ViewParams},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{egui, EguiContexts};
use bevy_pkv::PkvStore;
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

pub struct DockTreePlugin;
impl Plugin for DockTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_docktree);
    }
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

#[derive(Deref, DerefMut, Resource, Serialize, Deserialize)]
pub struct DockTree(DockState<Tab>);
impl Default for DockTree {
    fn default() -> Self {
        let mut tree = DockState::new(vec![Tab::Viewport]);
        tree.main_surface_mut()
            .split_left(NodeIndex::root(), 0.2, vec![Tab::View, Tab::Settings]);
        Self(tree)
    }
}

#[derive(Display, PartialEq, EnumIter, Serialize, Deserialize, Clone, Copy)]
pub enum Tab {
    Viewport,
    View,
    Settings,
}

// this tells egui how to render each tab
#[derive(SystemParam)]
pub struct TabViewer<'w, 's> {
    pub params: ParamSet<
        'w,
        's,
        (
            ViewportParams<'w, 's>,
            ViewParams<'w>,
            SettingsParams<'w, 's>,
        ),
    >,
}
impl egui_dock::TabViewer for TabViewer<'_, '_> {
    // each tab will be distinguished by a string - its name
    type Tab = Tab;
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // we can do different things inside the tab depending on its name
        match tab {
            Tab::Viewport => show_viewport_tab(ui, &mut self.params.p0()),
            Tab::View => show_view_tab(ui, &mut self.params.p1()),
            Tab::Settings => show_settings_tab(ui, &mut self.params.p2()),
        };
    }
    // show the title of the tab - the 'Tab' type already stores its title anyway
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }
}

#[derive(SystemParam)]
pub struct DockAreaParams<'w, 's> {
    params: ParamSet<'w, 's, (TabViewer<'w, 's>, ResMut<'w, AppSettings>)>,
    tree: ResMut<'w, DockTree>,

    contexts: EguiContexts<'w, 's>,
}

pub fn show_dock_area(mut p: DockAreaParams) {
    let ctx = p.contexts.ctx_mut();

    // show the actual dock area
    DockArea::new(&mut p.tree)
        .style(Style::from_egui(ctx.style().as_ref()))
        .show(ctx, &mut p.params.p0());

    if p.params.p1().reset_tree {
        *p.tree = DockTree::default();
        p.params.p1().reset_tree = false;
    }
}
