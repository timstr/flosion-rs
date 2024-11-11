use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    expression::expressiongraph::ExpressionGraphParameterId, stashing::StashingContext,
    uniqueid::UniqueId,
};

use super::{expressiongraph::ExpressionTarget, expressionnode::ExpressionNodeId};

pub struct ExpressionInputTag;

pub type ExpressionInputId = UniqueId<ExpressionInputTag>;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ExpressionInputLocation {
    NodeInput(ExpressionNodeId, ExpressionInputId),
    GraphResult(ExpressionInputId),
}

pub struct ExpressionInput {
    id: ExpressionInputId,
    target: Option<ExpressionTarget>,
    default_value: f32,
}

impl ExpressionInput {
    pub(crate) fn new(default_value: f32) -> ExpressionInput {
        ExpressionInput {
            id: ExpressionInputId::new_unique(),
            target: None,
            default_value,
        }
    }

    pub(crate) fn id(&self) -> ExpressionInputId {
        self.id
    }

    pub(crate) fn target(&self) -> Option<ExpressionTarget> {
        self.target
    }

    pub(crate) fn set_target(&mut self, target: Option<ExpressionTarget>) {
        self.target = target;
    }

    pub(crate) fn default_value(&self) -> f32 {
        self.default_value
    }
}

impl Stashable<StashingContext> for ExpressionInput {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.id.value() as _);
        match self.target {
            None => {
                stasher.u8(0);
            }
            Some(ExpressionTarget::Node(node_id)) => {
                stasher.u8(1);
                stasher.u64(node_id.value() as _);
            }
            Some(ExpressionTarget::Parameter(param_id)) => {
                stasher.u8(2);
                stasher.u64(param_id.value() as _);
            }
        }
        stasher.f32(self.default_value);
    }
}

impl Unstashable for ExpressionInput {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let id = ExpressionInputId::new(unstasher.u64()? as _);
        let target = match unstasher.u8()? {
            0 => None,
            1 => Some(ExpressionTarget::Node(ExpressionNodeId::new(
                unstasher.u64()? as _,
            ))),
            2 => Some(ExpressionTarget::Parameter(
                ExpressionGraphParameterId::new(unstasher.u64()? as _),
            )),
            _ => panic!(),
        };
        let default_value = unstasher.f32()?;

        Ok(ExpressionInput {
            id,
            target,
            default_value,
        })
    }
}

impl UnstashableInplace for ExpressionInput {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        // TODO: deduplicate this code

        let id = ExpressionInputId::new(unstasher.u64_always()? as _);
        let target = match unstasher.u8_always()? {
            0 => None,
            1 => Some(ExpressionTarget::Node(ExpressionNodeId::new(
                unstasher.u64_always()? as _,
            ))),
            2 => Some(ExpressionTarget::Parameter(
                ExpressionGraphParameterId::new(unstasher.u64_always()? as _),
            )),
            _ => panic!(),
        };
        let default_value = unstasher.f32_always()?;

        if unstasher.time_to_write() {
            *self = ExpressionInput {
                id,
                target,
                default_value,
            }
        }

        Ok(())
    }
}
