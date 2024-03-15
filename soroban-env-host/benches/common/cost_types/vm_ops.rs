use super::wasm_insn_exec::wasm_module_with_n_internal_funcs;
use crate::common::HostCostMeasurement;
use rand::rngs::StdRng;
use soroban_env_host::{
    cost_runner::{VmInstantiationRun, VmInstantiationSample},
    vm::{ParsedModule, VersionedContractCodeCostInputs},
    xdr, Host,
};
use std::rc::Rc;

// Protocol 20 coarse cost model.
pub(crate) struct VmInstantiationMeasure;

// This measures the cost of parsing wasm and/or instantiating a host::Vm on a
// variety of possible wasm modules, of different sizes.
macro_rules! impl_measurement_for_instantiation_cost_type {
    ($RUNNER:ty, $MEASURE:ty, $BUILD:ident, $USE_REFINED_INPUTS:expr, $MAGNITUDE:expr) => {
        impl HostCostMeasurement for $MEASURE {
            type Runner = $RUNNER;

            fn new_random_case(
                _host: &Host,
                _rng: &mut StdRng,
                input: u64,
            ) -> VmInstantiationSample {
                let id: xdr::Hash = [0; 32].into();
                let n = (Self::INPUT_BASE_SIZE + input * $MAGNITUDE) as usize;
                let wasm = $BUILD(n);
                #[allow(unused_mut)]
                let mut cost_inputs = VersionedContractCodeCostInputs::V0 {
                    wasm_bytes: wasm.len(),
                };
                #[cfg(feature = "next")]
                if $USE_REFINED_INPUTS {
                    cost_inputs = VersionedContractCodeCostInputs::V1(
                        soroban_env_host::vm::ParsedModule::extract_refined_contract_cost_inputs(
                            _host,
                            &wasm[..],
                        )
                        .unwrap(),
                    )
                }
                let module = Rc::new(
                    ParsedModule::new_with_isolated_engine(_host, &wasm, cost_inputs.clone())
                        .unwrap(),
                );
                VmInstantiationSample {
                    id: Some(id),
                    wasm,
                    module,
                }
            }
        }
    };
}

// Protocol 20 coarse unified, or protocol 21 coarse parse-phase cost model
impl_measurement_for_instantiation_cost_type!(
    VmInstantiationRun,
    VmInstantiationMeasure,
    wasm_module_with_n_internal_funcs,
    false,
    30
);

// Protocol 21 cost models.
#[cfg(feature = "next")]
pub(crate) use v21::*;
#[cfg(feature = "next")]
mod v21 {
    use super::super::wasm_insn_exec::{
        wasm_module_with_n_data_segments, wasm_module_with_n_elem_segments,
        wasm_module_with_n_exports, wasm_module_with_n_globals, wasm_module_with_n_imports,
        wasm_module_with_n_insns, wasm_module_with_n_internal_funcs,
        wasm_module_with_n_memory_pages, wasm_module_with_n_table_entries,
        wasm_module_with_n_types,
    };
    use super::*;
    use soroban_env_host::{
        cost_runner::{
            InstantiateWasmDataSegmentsRun, InstantiateWasmElemSegmentsRun,
            InstantiateWasmExportsRun, InstantiateWasmFunctionsRun, InstantiateWasmGlobalsRun,
            InstantiateWasmImportsRun, InstantiateWasmInstructionsRun,
            InstantiateWasmMemoryPagesRun, InstantiateWasmTableEntriesRun, InstantiateWasmTypesRun,
            ParseWasmDataSegmentsRun, ParseWasmElemSegmentsRun, ParseWasmExportsRun,
            ParseWasmFunctionsRun, ParseWasmGlobalsRun, ParseWasmImportsRun,
            ParseWasmInstructionsRun, ParseWasmMemoryPagesRun, ParseWasmTableEntriesRun,
            ParseWasmTypesRun, VmCachedInstantiationRun, VmInstantiationSample,
        },
        xdr, Host,
    };

    pub(crate) struct VmCachedInstantiationMeasure;

    pub(crate) struct ParseWasmInstructionsMeasure;
    pub(crate) struct ParseWasmFunctionsMeasure;
    pub(crate) struct ParseWasmGlobalsMeasure;
    pub(crate) struct ParseWasmTableEntriesMeasure;
    pub(crate) struct ParseWasmTypesMeasure;
    pub(crate) struct ParseWasmDataSegmentsMeasure;
    pub(crate) struct ParseWasmElemSegmentsMeasure;
    pub(crate) struct ParseWasmImportsMeasure;
    pub(crate) struct ParseWasmExportsMeasure;
    pub(crate) struct ParseWasmMemoryPagesMeasure;

    pub(crate) struct InstantiateWasmInstructionsMeasure;
    pub(crate) struct InstantiateWasmFunctionsMeasure;
    pub(crate) struct InstantiateWasmGlobalsMeasure;
    pub(crate) struct InstantiateWasmTableEntriesMeasure;
    pub(crate) struct InstantiateWasmTypesMeasure;
    pub(crate) struct InstantiateWasmDataSegmentsMeasure;
    pub(crate) struct InstantiateWasmElemSegmentsMeasure;
    pub(crate) struct InstantiateWasmImportsMeasure;
    pub(crate) struct InstantiateWasmExportsMeasure;
    pub(crate) struct InstantiateWasmMemoryPagesMeasure;

    // Protocol 21 coarse instantiation-phase cost model
    impl_measurement_for_instantiation_cost_type!(
        VmCachedInstantiationRun,
        VmCachedInstantiationMeasure,
        wasm_module_with_n_internal_funcs,
        false,
        30
    );

    // Protocol 21 refined cost model
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmInstructionsRun,
        ParseWasmInstructionsMeasure,
        wasm_module_with_n_insns,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmFunctionsRun,
        ParseWasmFunctionsMeasure,
        wasm_module_with_n_internal_funcs,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmGlobalsRun,
        ParseWasmGlobalsMeasure,
        wasm_module_with_n_globals,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmTableEntriesRun,
        ParseWasmTableEntriesMeasure,
        wasm_module_with_n_table_entries,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmTypesRun,
        ParseWasmTypesMeasure,
        wasm_module_with_n_types,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmDataSegmentsRun,
        ParseWasmDataSegmentsMeasure,
        wasm_module_with_n_data_segments,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmElemSegmentsRun,
        ParseWasmElemSegmentsMeasure,
        wasm_module_with_n_elem_segments,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmImportsRun,
        ParseWasmImportsMeasure,
        wasm_module_with_n_imports,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmExportsRun,
        ParseWasmExportsMeasure,
        wasm_module_with_n_exports,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        ParseWasmMemoryPagesRun,
        ParseWasmMemoryPagesMeasure,
        wasm_module_with_n_memory_pages,
        true,
        30
    );

    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmInstructionsRun,
        InstantiateWasmInstructionsMeasure,
        wasm_module_with_n_insns,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmFunctionsRun,
        InstantiateWasmFunctionsMeasure,
        wasm_module_with_n_internal_funcs,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmGlobalsRun,
        InstantiateWasmGlobalsMeasure,
        wasm_module_with_n_globals,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmTableEntriesRun,
        InstantiateWasmTableEntriesMeasure,
        wasm_module_with_n_table_entries,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmTypesRun,
        InstantiateWasmTypesMeasure,
        wasm_module_with_n_types,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmDataSegmentsRun,
        InstantiateWasmDataSegmentsMeasure,
        wasm_module_with_n_data_segments,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmElemSegmentsRun,
        InstantiateWasmElemSegmentsMeasure,
        wasm_module_with_n_elem_segments,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmImportsRun,
        InstantiateWasmImportsMeasure,
        wasm_module_with_n_imports,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmExportsRun,
        InstantiateWasmExportsMeasure,
        wasm_module_with_n_exports,
        true,
        30
    );
    impl_measurement_for_instantiation_cost_type!(
        InstantiateWasmMemoryPagesRun,
        InstantiateWasmMemoryPagesMeasure,
        wasm_module_with_n_memory_pages,
        true,
        30
    );
}
