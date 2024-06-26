use super::{
    AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object,
    RespawnPoint, StartPoint,
};
use bevy::prelude::*;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU32, Ordering},
};

pub struct OrderingPlugin;
impl Plugin for OrderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RefreshOrdering>()
            .ordering_setup::<StartPoint>()
            .ordering_setup::<EnemyPathPoint>()
            .ordering_setup::<ItemPathPoint>()
            .ordering_setup::<Checkpoint>()
            .ordering_setup::<RespawnPoint>()
            .ordering_setup::<Object>()
            .ordering_setup::<AreaPoint>()
            .ordering_setup::<KmpCamera>()
            .ordering_setup::<CannonPoint>()
            .ordering_setup::<BattleFinishPoint>();
    }
}

trait OrderingSetup {
    fn ordering_setup<T: Component>(&mut self) -> &mut Self;
}
impl OrderingSetup for App {
    fn ordering_setup<T: Component>(&mut self) -> &mut Self {
        self.init_resource::<NextOrderID<T>>()
            .add_systems(Update, refresh_order::<T>.run_if(on_event::<RefreshOrdering>()))
    }
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
    let mut order_ids = q.iter_mut().collect::<Vec<_>>();
    order_ids.sort_by(|x, y| x.0.cmp(&y.0));
    let mut id = -1;
    for (i, order_id) in order_ids.iter_mut().enumerate() {
        order_id.0 = i as u32;
        id = i as i32;
    }
    next_id.set((id + 1) as u32);
}
