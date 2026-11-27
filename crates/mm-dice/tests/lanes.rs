//! The 16 AC lanes, 10 to 25.

use mm_dice::{AcLanes, LANE_ACS, LANES};

#[test]
fn there_is_one_lane_per_ac_from_10_to_25() {
    assert_eq!(LANES, 16);
    assert_eq!(LANE_ACS, 10..=25);
    let lanes = AcLanes::from_fn(|ac| ac * 2);
    let acs: Vec<i32> = lanes.iter().map(|(ac, _)| ac).collect();
    assert_eq!(acs, (10..=25).collect::<Vec<_>>());
    assert!(lanes.iter().all(|(ac, &v)| v == ac * 2));
}

#[test]
fn a_lane_is_found_by_its_ac() {
    let lanes = AcLanes::from_fn(|ac| ac - 10);
    assert_eq!(lanes.get(10), Some(&0));
    assert_eq!(lanes.get(25), Some(&15));
    assert_eq!(lanes[17], 7);
    assert_eq!(lanes.get(9), None);
    assert_eq!(lanes.get(26), None);
}

#[test]
fn map_keeps_each_value_in_its_lane() {
    let lanes = AcLanes::from_fn(|ac| ac).map(|v| v + 100);
    assert_eq!(lanes[10], 110);
    assert_eq!(lanes[25], 125);
}
