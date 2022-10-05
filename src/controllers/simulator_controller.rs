use super::app::App;
use crate::{
    models::domain::system::LendingSystem,
    types::{Model, View},
    views::simulator_view::SimulatorView,
};
use shared::controller;

#[controller(SimulatorView)]
pub struct SimulatorController<M, V>
where
    M: Model + LendingSystem,
    V: View + SimulatorView,
{
    model: M,
    view: V,
}

impl<M, V> App<M> for SimulatorController<M, V>
where
    M: Model + LendingSystem,
    V: View + SimulatorView,
{
    fn run(&mut self, sys: M) -> M {
        todo!()
    }
}