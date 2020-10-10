use crate::bytes::Bytes;
use crate::error::Error;
use crate::smt::{Store, H256, SMT};
use crate::syscalls::{L2Syscalls, RunResult};
use crate::State;
use gw_types::{
    core::CallType,
    packed::{BlockInfo, CallContext, L2Block, RawL2Block, RawL2Transaction},
    prelude::*,
};
use lazy_static::lazy_static;
use std::collections::HashMap;

use ckb_vm::{
    machine::asm::{AsmCoreMachine, AsmMachine},
    DefaultMachineBuilder,
};

lazy_static! {
    static ref VALIDATOR: Bytes = include_bytes!("../../c/build/validator").to_vec().into();
    static ref GENERATOR: Bytes = include_bytes!("../../c/build/generator").to_vec().into();
}

pub struct Generator {
    generator: Bytes,
    validator: Bytes,
    contracts_by_code_hash: HashMap<[u8; 32], Bytes>,
}

impl Generator {
    pub fn new(contracts_by_code_hash: HashMap<[u8; 32], Bytes>) -> Self {
        Generator {
            generator: GENERATOR.clone(),
            validator: VALIDATOR.clone(),
            contracts_by_code_hash,
        }
    }

    /// Apply l2block state transition
    ///
    /// Notice:
    /// This function do not verify the block and transactions signature.
    /// The caller is supposed to do the verification.
    pub fn apply_block_state<S: Store<H256>>(
        &self,
        tree: &mut SMT<S>,
        block: &L2Block,
    ) -> Result<(), Error> {
        let raw_block = block.raw();
        if raw_block.submit_transactions().to_opt().is_none() {
            return Ok(());
        }
        let block_info = get_block_info(&raw_block);
        for tx in block.transactions() {
            let raw_tx = tx.raw();
            // check nonce
            let expected_nonce = tree.get_nonce(raw_tx.from_id().unpack())? + 1;
            let actual_nonce: u32 = raw_tx.nonce().unpack();
            if actual_nonce != expected_nonce {
                return Err(Error::Nonce {
                    expected: expected_nonce,
                    actual: actual_nonce,
                });
            }
            // build call context
            let call_context = get_call_context(&raw_tx);
            let run_result = self.execute(tree, &block_info, &call_context)?;
            tree.apply(&run_result)?;
        }
        Ok(())
    }

    /// execute a layer2 tx
    pub fn execute<S: Store<H256>>(
        &self,
        tree: &SMT<S>,
        block_info: &BlockInfo,
        call_context: &CallContext,
    ) -> Result<RunResult, Error> {
        let mut run_result = RunResult::default();
        {
            let core_machine = Box::<AsmCoreMachine>::default();
            let machine_builder =
                DefaultMachineBuilder::new(core_machine).syscall(Box::new(L2Syscalls {
                    tree,
                    block_info: block_info,
                    call_context: call_context,
                    result: &mut run_result,
                    contracts_by_code_hash: &self.contracts_by_code_hash,
                }));
            let mut machine = AsmMachine::new(machine_builder.build(), None);
            let program_name = Bytes::from_static(b"generator");
            machine.load_program(&self.generator, &[program_name])?;
            let code = machine.run()?;
            if code != 0 {
                return Err(Error::InvalidExitCode(code).into());
            }
        }
        Ok(run_result)
    }
}

fn get_block_info(l2block: &RawL2Block) -> BlockInfo {
    BlockInfo::new_builder()
        .aggregator_id(l2block.aggregator_id())
        .number(l2block.number())
        .timestamp(l2block.timestamp())
        .build()
}

fn get_call_context(l2tx: &RawL2Transaction) -> CallContext {
    // NOTICE users only allowed to send HandleMessage CallType txs
    CallContext::new_builder()
        .args(l2tx.args())
        .call_type(CallType::HandleMessage.into())
        .from_id(l2tx.from_id())
        .to_id(l2tx.to_id())
        .build()
}