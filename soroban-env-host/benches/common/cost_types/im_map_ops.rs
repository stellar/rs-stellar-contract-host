use crate::common::{util, HostCostMeasurement};
use rand::{rngs::StdRng, seq::SliceRandom};
use soroban_env_host::{
    cost_runner::{ImMapEntryRun, ImMapEntrySample, ImMapNewRun},
    Host, MeteredOrdMap,
};

pub(crate) struct ImMapNewMeasure;
// Measures the overhead cost of creating a new map, without inserting into the host storage.
impl HostCostMeasurement for ImMapNewMeasure {
    type Runner = ImMapNewRun;

    fn new_random_case(_host: &Host, _rng: &mut StdRng, _input: u64) {
        ()
    }
}

pub(crate) struct ImMapEntryMeasure;
// Measures the costs of accessing maps of varying sizes. It should have zero
// memory cost and logarithmic cpu cost, which we will approximate as constant
// with an upper bound on map size. The input value is the size of the map.
impl HostCostMeasurement for ImMapEntryMeasure {
    type Runner = ImMapEntryRun;

    fn new_random_case(host: &Host, rng: &mut StdRng, input: u64) -> ImMapEntrySample {
        let input = input * 100;
        let mut keys: Vec<_> = util::to_rawval_u32(0..(input as u32)).collect();
        let om = keys.iter().cloned().zip(keys.iter().cloned()).collect();
        let map: MeteredOrdMap<_, _, _> = MeteredOrdMap::from_map(om, host).unwrap();
        keys.shuffle(rng);
        ImMapEntrySample { map, keys }
    }
}
