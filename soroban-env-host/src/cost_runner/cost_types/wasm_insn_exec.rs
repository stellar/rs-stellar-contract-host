use crate::{cost_runner::CostRunner, xdr::ContractCostType, xdr::ScVec, RawVal, Symbol, Vm};
use std::{hint::black_box, rc::Rc};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// This is a subset of WASM instructions we are interested in for calibration.
/// Only interested in i64 numeric type (ignores u64, i(u)32, F32(64), unsigned).
pub enum WasmInsnType {
    LocalGet,
    LocalSet,
    LocalTee,
    Br,
    BrIfEqz,
    BrIfNez,
    ReturnIfNez,
    BrTable,
    Unreachable,
    Return,
    // Both `CallLocal` and `CallImport` invokes the same underneath Wasm instruction
    // `Call` but one calls a local function the other calls the imported one. Their
    // costs are slightly different.
    CallLocal,
    CallImport,
    CallIndirect,
    Drop,
    Select,
    GlobalGet,
    GlobalSet,
    I64Load,
    I64Load8S,
    I64Load16S,
    I64Load32S,
    I64Store,
    I64Store8,
    I64Store16,
    I64Store32,
    MemorySize,
    MemoryGrow,
    Const,
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64GtS,
    I64LeS,
    I64GeS,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64RemS,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64Rotl,
    I64Rotr,
}

impl WasmInsnType {
    pub fn variants() -> std::slice::Iter<'static, WasmInsnType> {
        static VARIANTS: &'static [WasmInsnType] = &[
            WasmInsnType::LocalGet,
            WasmInsnType::LocalSet,
            WasmInsnType::LocalTee,
            WasmInsnType::Br,
            WasmInsnType::BrIfEqz,
            WasmInsnType::BrIfNez,
            WasmInsnType::ReturnIfNez,
            WasmInsnType::BrTable,
            WasmInsnType::Unreachable,
            WasmInsnType::Return,
            WasmInsnType::CallLocal,
            WasmInsnType::CallImport,
            WasmInsnType::CallIndirect,
            WasmInsnType::Drop,
            WasmInsnType::Select,
            WasmInsnType::GlobalGet,
            WasmInsnType::GlobalSet,
            WasmInsnType::I64Load,
            WasmInsnType::I64Load8S,
            WasmInsnType::I64Load16S,
            WasmInsnType::I64Load32S,
            WasmInsnType::I64Store,
            WasmInsnType::I64Store8,
            WasmInsnType::I64Store16,
            WasmInsnType::I64Store32,
            WasmInsnType::MemorySize,
            WasmInsnType::MemoryGrow,
            WasmInsnType::Const,
            WasmInsnType::I64Eqz,
            WasmInsnType::I64Eq,
            WasmInsnType::I64Ne,
            WasmInsnType::I64LtS,
            WasmInsnType::I64GtS,
            WasmInsnType::I64LeS,
            WasmInsnType::I64GeS,
            WasmInsnType::I64Clz,
            WasmInsnType::I64Ctz,
            WasmInsnType::I64Popcnt,
            WasmInsnType::I64Add,
            WasmInsnType::I64Sub,
            WasmInsnType::I64Mul,
            WasmInsnType::I64DivS,
            WasmInsnType::I64RemS,
            WasmInsnType::I64And,
            WasmInsnType::I64Or,
            WasmInsnType::I64Xor,
            WasmInsnType::I64Shl,
            WasmInsnType::I64ShrS,
            WasmInsnType::I64Rotl,
            WasmInsnType::I64Rotr,
        ];
        VARIANTS.iter()
    }
}

#[derive(Clone)]
pub struct WasmInsnSample {
    pub vm: Rc<Vm>,
    pub insns: u64,
    pub overhead: u64,
}

#[derive(Clone)]
pub struct WasmInsnExecSample {
    pub args: ScVec,
    pub vm: Rc<Vm>,
}

const TEST_SYM: Symbol = match Symbol::try_from_small_str("test") {
    Ok(s) => s,
    _ => panic!(),
};

macro_rules! impl_wasm_insn_runner {
    ($runner:ident, $insn:ident) => {
        pub struct $runner;

        impl $runner {
            pub const INSN_TYPE: WasmInsnType = WasmInsnType::$insn;
        }

        impl CostRunner for $runner {
            const COST_TYPE: ContractCostType = ContractCostType::WasmInsnExec;
            type SampleType = WasmInsnSample;
            type RecycledType = (Option<RawVal>, Self::SampleType);

            fn run_iter(
                host: &crate::Host,
                _iter: u64,
                sample: Self::SampleType,
            ) -> Self::RecycledType {
                let rv = black_box(
                    sample
                        .vm
                        .invoke_function_raw(host, &TEST_SYM, &[])
                        .unwrap_or_default(),
                );
                (Some(rv), sample)
            }

            fn run_baseline_iter(
                host: &crate::Host,
                _iter: u64,
                base_sample: Self::SampleType,
            ) -> Self::RecycledType {
                let rv = black_box(
                    base_sample
                        .vm
                        .invoke_function_raw(host, &TEST_SYM, &[])
                        .unwrap_or_default(),
                );
                (Some(rv), base_sample)
            }
        }
    };
}

impl_wasm_insn_runner!(ConstRun, Const);
impl_wasm_insn_runner!(DropRun, Drop);
impl_wasm_insn_runner!(SelectRun, Select);
impl_wasm_insn_runner!(BrRun, Br);
impl_wasm_insn_runner!(BrTableRun, BrTable);
impl_wasm_insn_runner!(LocalGetRun, LocalGet);
impl_wasm_insn_runner!(LocalSetRun, LocalSet);
impl_wasm_insn_runner!(LocalTeeRun, LocalTee);
impl_wasm_insn_runner!(CallLocalRun, CallLocal);
impl_wasm_insn_runner!(CallImportRun, CallImport);
impl_wasm_insn_runner!(CallIndirectRun, CallIndirect);
impl_wasm_insn_runner!(GlobalGetRun, GlobalGet);
impl_wasm_insn_runner!(GlobalSetRun, GlobalSet);
impl_wasm_insn_runner!(I64StoreRun, I64Store);
impl_wasm_insn_runner!(I64LoadRun, I64Load);
impl_wasm_insn_runner!(I64Load8SRun, I64Load8S);
impl_wasm_insn_runner!(I64Load16SRun, I64Load16S);
impl_wasm_insn_runner!(I64Load32SRun, I64Load32S);
impl_wasm_insn_runner!(I64Store8Run, I64Store8);
impl_wasm_insn_runner!(I64Store16Run, I64Store16);
impl_wasm_insn_runner!(I64Store32Run, I64Store32);
impl_wasm_insn_runner!(MemorySizeRun, MemorySize);
impl_wasm_insn_runner!(MemoryGrowRun, MemoryGrow);
impl_wasm_insn_runner!(I64ClzRun, I64Clz);
impl_wasm_insn_runner!(I64CtzRun, I64Ctz);
impl_wasm_insn_runner!(I64PopcntRun, I64Popcnt);
impl_wasm_insn_runner!(I64EqzRun, I64Eqz);
impl_wasm_insn_runner!(I64EqRun, I64Eq);
impl_wasm_insn_runner!(I64NeRun, I64Ne);
impl_wasm_insn_runner!(I64LtSRun, I64LtS);
impl_wasm_insn_runner!(I64GtSRun, I64GtS);
impl_wasm_insn_runner!(I64LeSRun, I64LeS);
impl_wasm_insn_runner!(I64GeSRun, I64GeS);
impl_wasm_insn_runner!(I64AddRun, I64Add);
impl_wasm_insn_runner!(I64SubRun, I64Sub);
impl_wasm_insn_runner!(I64MulRun, I64Mul);
impl_wasm_insn_runner!(I64DivSRun, I64DivS);
impl_wasm_insn_runner!(I64RemSRun, I64RemS);
impl_wasm_insn_runner!(I64AndRun, I64And);
impl_wasm_insn_runner!(I64OrRun, I64Or);
impl_wasm_insn_runner!(I64XorRun, I64Xor);
impl_wasm_insn_runner!(I64ShlRun, I64Shl);
impl_wasm_insn_runner!(I64ShrSRun, I64ShrS);
impl_wasm_insn_runner!(I64RotlRun, I64Rotl);
impl_wasm_insn_runner!(I64RotrRun, I64Rotr);
