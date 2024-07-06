use super::{
    sections::add_for_all_components, AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint,
    ItemPathPoint, KmpCamera, Object, RespawnPoint, StartPoint,
};
use bevy::prelude::*;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU32, Ordering},
};

pub fn ordering_plugin(app: &mut App) {
    app.add_event::<RefreshOrdering>()
        .add_plugins(add_for_all_components!(setup_ordering));
}

fn setup_ordering<T: Component>(app: &mut App) {
    app.init_resource::<NextOrderID<T>>()
        .add_systems(Update, refresh_order::<T>.run_if(on_event::<RefreshOrdering>()));
}

#[derive(Component, Default, PartialEq, Eq, PartialOrd, Ord, Deref, DerefMut)]
pub struct OrderID(pub u32);

#[derive(Resource)]
pub struct NextOrderID<T> {
    pub id: AtomicU32,
    _p: PhantomData<T>,
}
impl<T: Component> Default for NextOrderID<T> {
    fn default() -> Self {
        Self {
            id: AtomicU32::new(0),
            _p: PhantomData,
        }
    }
}
impl<T: Component> NextOrderID<T> {
    pub fn set(&self, id: impl Into<u32>) {
        self.id.store(id.into(), Ordering::Relaxed);
    }
    pub fn get(&self) -> u32 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }
}

#[derive(Event, Default)]
pub struct RefreshOrdering;

pub fn refresh_order<T: Component>(mut q: Query<&mut OrderID, With<T>>, next_id: Res<NextOrderID<T>>) {
    let mut id = 0u32;
    for (i, mut order_id) in q.iter_mut().sort_by_key::<&OrderID, _>(|x| *x).enumerate() {
        order_id.0 = i as u32;
        id = i as u32 + 1;
    }
    next_id.set(id);
}
