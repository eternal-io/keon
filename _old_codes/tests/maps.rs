use std::collections::BTreeMap;
mod util;

#[test]
fn roundtrips() {
    let map = BTreeMap::<i32, i32>::from_iter(vec![(1, 2), (3, 4)].into_iter());

    util::rt_min(&map, "{1=>2,3=>4}").unwrap();
    util::rt_pre(&map, "{\n    1 => 2,\n    3 => 4,\n}").unwrap();

    let mut mapmap = BTreeMap::<BTreeMap<i32, i32>, BTreeMap<i32, i32>>::new();
    mapmap.insert(
        BTreeMap::from_iter(vec![(1, 2), (3, 4)].into_iter()),
        BTreeMap::from_iter(vec![(5, 6), (7, 8)].into_iter()),
    );

    util::rt_min(&mapmap, "{{1=>2,3=>4}=>{5=>6,7=>8}}").unwrap();
    util::rt_pre(
        &mapmap,
        "{
    {
        1 => 2,
        3 => 4,
    } => {
        5 => 6,
        7 => 8,
    },
}",
    )
    .unwrap();
}
