use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorContent {
    Goat,
    Car,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorStatus {
    Closed,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Door {
    pub id: usize,
    pub content: DoorContent,
    pub status: DoorStatus,
}

pub fn setup_doors() -> [Door; 3] {
    let mut rng = rand::thread_rng();
    let car_index = rng.gen_range(0..3);
    [
        Door {
            id: 1,
            content: if car_index == 0 { DoorContent::Car } else { DoorContent::Goat },
            status: DoorStatus::Closed,
        },
        Door {
            id: 2,
            content: if car_index == 1 { DoorContent::Car } else { DoorContent::Goat },
            status: DoorStatus::Closed,
        },
        Door {
            id: 3,
            content: if car_index == 2 { DoorContent::Car } else { DoorContent::Goat },
            status: DoorStatus::Closed,
        },
    ]
}

pub fn host_reveal(doors: &[Door; 3], player_choice: usize) -> usize {
    let mut rng = rand::thread_rng();
    let available: Vec<usize> = doors
        .iter()
        .filter(|d| d.id != player_choice && d.content == DoorContent::Goat)
        .map(|d| d.id)
        .collect();
    available[rng.gen_range(0..available.len())]
}

pub fn resolve_game(doors: &[Door; 3], final_choice: usize) -> (bool, [Door; 3]) {
    let win = doors.iter().find(|d| d.id == final_choice).map_or(false, |d| d.content == DoorContent::Car);
    let mut revealed = *doors;
    for d in revealed.iter_mut() {
        d.status = DoorStatus::Open;
    }
    (win, revealed)
}
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_batch_wasm(count: u64) -> Vec<u64> {
    let (sw, stw) = run_batch(count);
    vec![sw, stw]
}

/// Fast batch simulation — returns (switch_wins, stick_wins)
pub fn run_batch(count: u64) -> (u64, u64) {
    let mut switch_wins: u64 = 0;
    let mut stick_wins: u64 = 0;
    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let car = rng.gen_range(0..3u8);
        let pick = rng.gen_range(0..3u8);

        if pick == car {
            stick_wins += 1;
        } else {
            switch_wins += 1;
        }
    }

    (switch_wins, stick_wins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_doors_has_one_car_two_goats() {
        for _ in 0..100 {
            let doors = setup_doors();
            let cars = doors.iter().filter(|d| d.content == DoorContent::Car).count();
            let goats = doors.iter().filter(|d| d.content == DoorContent::Goat).count();
            assert_eq!(cars, 1);
            assert_eq!(goats, 2);
        }
    }

    #[test]
    fn host_never_reveals_car_or_player_choice() {
        for _ in 0..200 {
            let doors = setup_doors();
            let player = doors[0].id;
            let revealed = host_reveal(&doors, player);
            assert_ne!(revealed, player);
            let revealed_door = doors.iter().find(|d| d.id == revealed).unwrap();
            assert_eq!(revealed_door.content, DoorContent::Goat);
        }
    }

    #[test]
    fn batch_simulation_converges() {
        let (sw, stw) = run_batch(100_000);
        let total = 100_000f64;
        let switch_rate = sw as f64 / total;
        let stick_rate = stw as f64 / total;
        // Switch should be ~66.7%, stick ~33.3%
        assert!((switch_rate - 0.667).abs() < 0.02, "switch rate: {switch_rate}");
        assert!((stick_rate - 0.333).abs() < 0.02, "stick rate: {stick_rate}");
    }
}
