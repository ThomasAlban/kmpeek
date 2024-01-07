mod mode;
mod settings;
mod view;
mod viewport;

pub use settings::*;
pub use view::*;
pub use viewport::*;

use self::mode::ShowModeTab;

use super::{
    app_state::AppSettings,
    tabs::{ShowSettingsTab, ShowViewTab, ShowViewportTab},
    update_ui::UiSection,
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
            .split_left(NodeIndex::root(), 0.2, vec![Tab::Mode]);
        Self(tree)
    }
}

pub trait UiTabSection {
    fn show(&mut self, ui: &mut egui::Ui);
}

#[derive(Display, PartialEq, EnumIter, Serialize, Deserialize, Clone, Copy)]
pub enum Tab {
    Viewport,
    Mode,
    View,
    Settings,
}

// this tells egui how to render each tab
#[derive(SystemParam)]
pub struct TabViewer<'w, 's> {
    pub p: ParamSet<
        'w,
        's,
        (
            ShowViewportTab<'w, 's>,
            ShowModeTab<'w, 's>,
            ShowViewTab<'w>,
            ShowSettingsTab<'w, 's>,
        ),
    >,
}
impl egui_dock::TabViewer for TabViewer<'_, '_> {
    // each tab will be distinguished by a string - its name
    type Tab = Tab;
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // we can do different things inside the tab depending on its name
        match tab {
            Tab::Viewport => self.p.p0().show(ui),
            Tab::Mode => self.p.p1().show(ui),
            Tab::View => self.p.p2().show(ui),
            Tab::Settings => self.p.p3().show(ui),
        };
    }
    // show the title of the tab - the 'Tab' type already stores its title anyway
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }
}

#[derive(SystemParam)]
pub struct ShowDockArea<'w, 's> {
    params: ParamSet<'w, 's, (TabViewer<'w, 's>, ResMut<'w, AppSettings>)>,
    tree: ResMut<'w, DockTree>,

    contexts: EguiContexts<'w, 's>,
}
impl UiSection for ShowDockArea<'_, '_> {
    fn show(&mut self) {
        let ctx = self.contexts.ctx_mut();

        let style = Style::from_egui(ctx.style().as_ref());

        // show the actual dock area
        DockArea::new(&mut self.tree)
            .style(style)
            .show(ctx, &mut self.params.p0());

        if self.params.p1().reset_tree {
            *self.tree = DockTree::default();
            self.params.p1().reset_tree = false;
        }
    }
}
