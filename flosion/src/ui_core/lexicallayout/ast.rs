use std::cell::Cell;

use eframe::egui;
use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::{
    expression::{
        expressiongraph::{ExpressionGraphParameterId, ExpressionTarget},
        expressioninput::ExpressionInputId,
        expressionnode::ExpressionNodeId,
    },
    uniqueid::UniqueId,
};

#[derive(Clone)]
pub(crate) struct ASTPath {
    steps: Vec<usize>,
}

impl ASTPath {
    #[cfg(test)]
    pub(super) fn new(steps: Vec<usize>) -> ASTPath {
        ASTPath { steps }
    }

    pub(super) fn new_at_beginning() -> ASTPath {
        ASTPath { steps: Vec::new() }
    }

    pub(crate) fn new_at_end_of(value: &ASTNode) -> ASTPath {
        let mut steps = Vec::new();
        fn visitor(node: &ASTNode, steps: &mut Vec<usize>) {
            let Some(inode) = node.as_internal_node() else {
                return;
            };
            let n = inode.num_children();
            if n == 0 {
                return;
            }
            let i = n - 1;
            steps.push(i);
            visitor(inode.get_child(i), steps);
        }
        visitor(value, &mut steps);
        ASTPath { steps }
    }

    pub(super) fn is_at_beginning(&self) -> bool {
        self.steps.is_empty()
    }

    pub(super) fn is_at_end_of(&self, value: &ASTNode) -> bool {
        fn visitor(node: &ASTNode, steps: &[usize]) -> bool {
            let Some((i, next_steps)) = steps.split_first() else {
                return match node.as_internal_node() {
                    Some(inode) => inode.num_children() == 0,
                    None => true,
                };
            };
            let inode = node.as_internal_node().unwrap();
            if *i + 1 < node.num_children() {
                return false;
            }
            visitor(inode.get_child(*i), next_steps)
        }
        visitor(value, self.steps())
    }

    pub(super) fn steps(&self) -> &[usize] {
        &self.steps
    }

    pub(super) fn go_left(&mut self, tree: &ASTNode) {
        let Some(last_step) = self.steps.pop() else {
            return;
        };
        if last_step > 0 {
            self.steps.push(last_step - 1);
            loop {
                let node = tree.get_along_path(&self.steps);
                let num_children = node.num_children();
                if num_children > 0 {
                    self.steps.push(num_children - 1);
                } else {
                    break;
                }
            }
        }
    }

    pub(super) fn go_right(&mut self, tree: &ASTNode) {
        if tree.get_along_path(&self.steps).num_children() > 0 {
            self.steps.push(0);
            return;
        }
        loop {
            let Some(last_step) = self.steps.pop() else {
                break;
            };
            let parent = tree.get_along_path(&self.steps);
            let num_siblings = parent.num_children();
            let next_step = last_step + 1;
            if next_step < num_siblings {
                self.steps.push(next_step);
                return;
            }
        }
        loop {
            let node = tree.get_along_path(&self.steps);
            let num_children = node.num_children();
            if num_children > 0 {
                self.steps.push(num_children - 1);
            } else {
                return;
            }
        }
    }

    pub(super) fn go_into(&mut self, index: usize) {
        self.steps.push(index);
    }
}

impl Stashable for ASTPath {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_u64_iter(self.steps.iter().map(|i| *i as u64));
    }
}

impl Unstashable for ASTPath {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(ASTPath {
            steps: unstasher.array_of_u64_iter()?.map(|i| i as usize).collect(),
        })
    }
}

pub struct VariableTag;

pub(crate) type VariableId = UniqueId<VariableTag>;

pub(crate) struct VariableDefinition {
    pub(super) id: VariableId,
    pub(super) name: String,
    pub(super) name_rect: Cell<egui::Rect>,
    pub(super) value: ASTNode,
}

impl VariableDefinition {
    pub(super) fn new(id: VariableId, name: String, value: ASTNode) -> VariableDefinition {
        VariableDefinition {
            id,
            name,
            name_rect: Cell::new(egui::Rect::NOTHING),
            value,
        }
    }

    pub(crate) fn id(&self) -> VariableId {
        self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn name_rect(&self) -> egui::Rect {
        self.name_rect.get()
    }

    pub(crate) fn value(&self) -> &ASTNode {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut ASTNode {
        &mut self.value
    }
}

impl Stashable for VariableDefinition {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.u64(self.id.value() as _);
        stasher.string(&self.name);
        // skipping name_rect, it will be regenerating when drawn
        stasher.object(&self.value);
    }
}

impl Unstashable for VariableDefinition {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let id = VariableId::new(unstasher.u64()? as _);
        let name = unstasher.string()?;
        let name_rect = Cell::new(egui::Rect::NOTHING);
        let value = unstasher.object()?;
        Ok(VariableDefinition {
            id,
            name,
            name_rect,
            value,
        })
    }
}

pub(crate) struct FinalExpression {
    pub(super) result_id: ExpressionInputId,
    pub(super) value: ASTNode,
}

impl FinalExpression {
    pub(crate) fn new(result_id: ExpressionInputId, value: ASTNode) -> FinalExpression {
        FinalExpression { result_id, value }
    }

    pub(crate) fn value(&self) -> &ASTNode {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut ASTNode {
        &mut self.value
    }

    pub(crate) fn result_id(&self) -> ExpressionInputId {
        self.result_id
    }
}

impl Stashable for FinalExpression {
    fn stash(&self, stasher: &mut Stasher) {
        self.result_id.stash(stasher);
        self.value.stash(stasher);
    }
}

impl Unstashable for FinalExpression {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(FinalExpression {
            result_id: ExpressionInputId::unstash(unstasher)?,
            value: ASTNode::unstash(unstasher)?,
        })
    }
}

pub(super) fn find_variable_definition(
    id: VariableId,
    definitions: &[VariableDefinition],
) -> Option<&VariableDefinition> {
    definitions.iter().find(|defn| defn.id() == id)
}

// Given a variable id, finds that variable's definition and returns it along
// with all variable definitions preceding it. Those form the set of variables
// the the found definition is allowed to refer to.
pub(super) fn find_variable_definition_and_scope(
    id: VariableId,
    definitions: &[VariableDefinition],
) -> Option<(&VariableDefinition, &[VariableDefinition])> {
    let i = definitions.iter().position(|defn| defn.id() == id);
    let Some(i) = i else {
        return None;
    };
    Some((&definitions[i], &definitions[..i]))
}

#[derive(Copy, Clone)]
pub(crate) enum ASTRoot {
    VariableDefinition(VariableId),
    FinalExpression(ExpressionInputId),
}

#[derive(Copy, Clone)]
pub(crate) enum ASTNodeParent<'a> {
    VariableDefinition(VariableId),
    FinalExpression(ExpressionInputId),
    InternalNode(&'a InternalASTNode, usize),
}

// TODO: rename, this has many uses beyond just building a path
// and possibly more uses than a path alone
#[derive(Clone, Copy)]
pub(crate) enum ASTPathBuilder<'a> {
    Root(ASTRoot),
    ChildOf(&'a InternalASTNode, usize),
}

impl<'a> ASTPathBuilder<'a> {
    pub(super) fn parent_node(&self) -> ASTNodeParent {
        match self {
            ASTPathBuilder::Root(ASTRoot::VariableDefinition(id)) => {
                ASTNodeParent::VariableDefinition(*id)
            }
            ASTPathBuilder::Root(ASTRoot::FinalExpression(i)) => ASTNodeParent::FinalExpression(*i),
            ASTPathBuilder::ChildOf(n, i) => ASTNodeParent::InternalNode(n, *i),
        }
    }

    pub(super) fn push(
        &'a self,
        parent: &'a InternalASTNode,
        child_index: usize,
    ) -> ASTPathBuilder<'a> {
        ASTPathBuilder::ChildOf(parent, child_index)
    }
}

pub(super) enum ASTNodeValue {
    Empty,
    Internal(Box<InternalASTNode>),
    Variable(VariableId),
    Parameter(ExpressionGraphParameterId),
}

pub(crate) struct ASTNode {
    value: ASTNodeValue,
    rect: Cell<egui::Rect>,
}

impl ASTNode {
    pub(super) fn new(value: ASTNodeValue) -> ASTNode {
        ASTNode {
            value,
            rect: Cell::new(egui::Rect::NOTHING),
        }
    }

    pub(super) fn value(&self) -> &ASTNodeValue {
        &self.value
    }

    // The expression graph node or parameter that this ASTNode directly corresponds to.
    // Variables are not looked up and do not correspond directly to a part of the graph.
    pub(super) fn direct_target(&self) -> Option<ExpressionTarget> {
        match &self.value {
            ASTNodeValue::Internal(node) => Some(node.expression_node_id().into()),
            ASTNodeValue::Parameter(giid) => Some((*giid).into()),
            _ => None,
        }
    }

    // The expression graph node or parameter that this ASTNode indirectly corresponds to.
    // Variables are looked up recursively in case of aliased definitions.
    pub(super) fn indirect_target(
        &self,
        definitions: &[VariableDefinition],
    ) -> Option<ExpressionTarget> {
        match &self.value {
            ASTNodeValue::Internal(node) => Some(node.expression_node_id().into()),
            ASTNodeValue::Variable(id) => {
                let (defn, previous_defns) =
                    find_variable_definition_and_scope(*id, definitions).unwrap();
                defn.value().indirect_target(previous_defns)
            }
            ASTNodeValue::Parameter(giid) => Some((*giid).into()),
            _ => None,
        }
    }

    pub(super) fn as_internal_node(&self) -> Option<&InternalASTNode> {
        match &self.value {
            ASTNodeValue::Internal(n) => Some(&*n),
            _ => None,
        }
    }

    pub(super) fn as_internal_node_mut(&mut self) -> Option<&mut InternalASTNode> {
        match &mut self.value {
            ASTNodeValue::Internal(n) => Some(&mut *n),
            _ => None,
        }
    }

    pub(super) fn num_children(&self) -> usize {
        self.as_internal_node()
            .and_then(|n| Some(n.num_children()))
            .unwrap_or(0)
    }

    pub(crate) fn rect(&self) -> egui::Rect {
        self.rect.get()
    }

    pub(super) fn set_rect(&self, rect: egui::Rect) {
        self.rect.set(rect);
    }

    pub(super) fn get_along_path(&self, path: &[usize]) -> &ASTNode {
        if let Some((head, tail)) = path.split_first() {
            self.as_internal_node()
                .unwrap()
                .get_child(*head)
                .get_along_path(tail)
        } else {
            self
        }
    }

    pub(super) fn set_along_path(&mut self, path: &[usize], value: ASTNode) {
        if let Some((head, tail)) = path.split_first() {
            self.as_internal_node_mut()
                .unwrap()
                .get_child_mut(*head)
                .set_along_path(tail, value)
        } else {
            *self = value;
        }
    }

    pub(super) fn find_parent_along_path(
        &self,
        path: &[usize],
    ) -> Option<(&InternalASTNode, usize)> {
        if let Some((head, tail)) = path.split_first() {
            let internal_node = self.as_internal_node().unwrap();
            if tail.is_empty() {
                Some((internal_node, *head))
            } else {
                internal_node.get_child(*head).find_parent_along_path(tail)
            }
        } else {
            None
        }
    }

    pub(super) fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(
        &self,
        path: ASTPathBuilder,
        f: &mut F,
    ) {
        f(self, path);
        if let ASTNodeValue::Internal(node) = &self.value {
            node.visit(path, f);
        }
    }

    pub(super) fn visit_mut<F: FnMut(&mut ASTNode, ASTPathBuilder)>(
        &mut self,
        path: ASTPathBuilder,
        f: &mut F,
    ) {
        f(self, path);
        if let ASTNodeValue::Internal(node) = &mut self.value {
            node.visit_mut(path, f);
        }
    }
}

pub(super) enum InternalASTNodeValue {
    Prefix(ExpressionNodeId, ASTNode),
    Infix(ASTNode, ExpressionNodeId, ASTNode),
    Postfix(ASTNode, ExpressionNodeId),
    Function(ExpressionNodeId, Vec<ASTNode>),
}

pub(crate) struct InternalASTNode {
    value: InternalASTNodeValue,
    self_rect: Cell<egui::Rect>,
}

impl InternalASTNode {
    pub(super) fn new(value: InternalASTNodeValue) -> InternalASTNode {
        InternalASTNode {
            value,
            self_rect: Cell::new(egui::Rect::NOTHING),
        }
    }

    pub(super) fn value(&self) -> &InternalASTNodeValue {
        &self.value
    }

    pub(super) fn value_mut(&mut self) -> &mut InternalASTNodeValue {
        &mut self.value
    }

    pub(super) fn expression_node_id(&self) -> ExpressionNodeId {
        match &self.value {
            InternalASTNodeValue::Prefix(id, _) => *id,
            InternalASTNodeValue::Infix(_, id, _) => *id,
            InternalASTNodeValue::Postfix(_, id) => *id,
            InternalASTNodeValue::Function(id, _) => *id,
        }
    }

    pub(super) fn num_children(&self) -> usize {
        match &self.value {
            InternalASTNodeValue::Prefix(_, _) => 1,
            InternalASTNodeValue::Infix(_, _, _) => 2,
            InternalASTNodeValue::Postfix(_, _) => 1,
            InternalASTNodeValue::Function(_, c) => c.len(),
        }
    }

    pub(super) fn set_self_rect(&self, rect: egui::Rect) {
        self.self_rect.set(rect);
    }

    pub(super) fn get_child(&self, index: usize) -> &ASTNode {
        match (index, &self.value) {
            (0, InternalASTNodeValue::Prefix(_, c)) => c,
            (0, InternalASTNodeValue::Infix(c, _, _)) => c,
            (1, InternalASTNodeValue::Infix(_, _, c)) => c,
            (0, InternalASTNodeValue::Postfix(c, _)) => c,
            (i, InternalASTNodeValue::Function(_, cs)) => &cs[i],
            _ => panic!("Invalid child index"),
        }
    }

    pub(super) fn get_child_mut(&mut self, index: usize) -> &mut ASTNode {
        match (index, &mut self.value) {
            (0, InternalASTNodeValue::Prefix(_, c)) => c,
            (0, InternalASTNodeValue::Infix(c, _, _)) => c,
            (1, InternalASTNodeValue::Infix(_, _, c)) => c,
            (0, InternalASTNodeValue::Postfix(c, _)) => c,
            (i, InternalASTNodeValue::Function(_, cs)) => &mut cs[i],
            _ => panic!("Invalid child index"),
        }
    }

    fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(&self, path: ASTPathBuilder, f: &mut F) {
        match &self.value {
            InternalASTNodeValue::Prefix(_, c) => c.visit(path.push(self, 0), f),
            InternalASTNodeValue::Infix(c1, _, c2) => {
                c1.visit(path.push(self, 0), f);
                c2.visit(path.push(self, 1), f)
            }
            InternalASTNodeValue::Postfix(c, _) => c.visit(path.push(self, 0), f),
            InternalASTNodeValue::Function(_, cs) => {
                for (i, c) in cs.iter().enumerate() {
                    c.visit(path.push(self, i), f);
                }
            }
        }
    }

    fn visit_mut<F: FnMut(&mut ASTNode, ASTPathBuilder)>(
        &mut self,
        path: ASTPathBuilder,
        f: &mut F,
    ) {
        // Swap self.value into a temporary in order to allow borrowing self
        let mut tmp_value = std::mem::replace(
            &mut self.value,
            InternalASTNodeValue::Function(ExpressionNodeId::new(1), Vec::new()),
        );
        match &mut tmp_value {
            InternalASTNodeValue::Prefix(_, c) => c.visit_mut(path.push(self, 0), f),
            InternalASTNodeValue::Infix(c1, _, c2) => {
                c1.visit_mut(path.push(self, 0), f);
                c2.visit_mut(path.push(self, 1), f)
            }
            InternalASTNodeValue::Postfix(c, _) => c.visit_mut(path.push(self, 0), f),
            InternalASTNodeValue::Function(_, cs) => {
                for (i, c) in cs.iter_mut().enumerate() {
                    c.visit_mut(path.push(self, i), f);
                }
            }
        }
        self.value = tmp_value;
    }
}

impl Stashable for ASTNode {
    fn stash(&self, stasher: &mut Stasher) {
        match &self.value {
            ASTNodeValue::Empty => {
                stasher.u8(0);
            }
            ASTNodeValue::Internal(internal_astnode) => {
                stasher.u8(1);
                stasher.object(&**internal_astnode);
            }
            ASTNodeValue::Variable(var_id) => {
                stasher.u8(2);
                var_id.stash(stasher);
            }
            ASTNodeValue::Parameter(param_id) => {
                stasher.u8(3);
                param_id.stash(stasher);
            }
        }
        // Skipping rect, it will be regenerated when drawn
    }
}

impl Unstashable for ASTNode {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let value = match unstasher.u8()? {
            0 => ASTNodeValue::Empty,
            1 => ASTNodeValue::Internal(Box::new(unstasher.object()?)),
            2 => ASTNodeValue::Variable(VariableId::unstash(unstasher)?),
            3 => ASTNodeValue::Parameter(ExpressionGraphParameterId::unstash(unstasher)?),
            _ => panic!(),
        };

        Ok(ASTNode {
            value,
            rect: Cell::new(egui::Rect::NOTHING),
        })
    }
}

impl Stashable for InternalASTNode {
    fn stash(&self, stasher: &mut Stasher<()>) {
        match &self.value {
            InternalASTNodeValue::Prefix(node_id, astnode) => {
                stasher.u8(0);
                stasher.u64(node_id.value() as _);
                stasher.object(astnode);
            }
            InternalASTNodeValue::Infix(leftastnode, node_id, rightastnode) => {
                stasher.u8(1);
                stasher.object(leftastnode);
                stasher.u64(node_id.value() as _);
                stasher.object(rightastnode);
            }
            InternalASTNodeValue::Postfix(astnode, node_id) => {
                stasher.u8(2);
                stasher.object(astnode);
                stasher.u64(node_id.value() as _);
            }
            InternalASTNodeValue::Function(node_id, vec) => {
                stasher.u8(3);
                stasher.u64(node_id.value() as _);
                stasher.array_of_objects_slice(&vec, Order::Ordered);
            }
        }
        // skipping self_rect, it will be regenerated when drawn
    }
}

impl Unstashable for InternalASTNode {
    fn unstash(unstasher: &mut Unstasher<()>) -> Result<Self, UnstashError> {
        let value = match unstasher.u8()? {
            0 => {
                let node_id = ExpressionNodeId::new(unstasher.u64()? as _);
                let astnode = unstasher.object()?;
                InternalASTNodeValue::Prefix(node_id, astnode)
            }
            1 => {
                let leftastnode = unstasher.object()?;
                let node_id = ExpressionNodeId::new(unstasher.u64()? as _);
                let rightastnode = unstasher.object()?;
                InternalASTNodeValue::Infix(leftastnode, node_id, rightastnode)
            }
            2 => {
                let astnode = unstasher.object()?;
                let node_id = ExpressionNodeId::new(unstasher.u64()? as _);
                InternalASTNodeValue::Postfix(astnode, node_id)
            }
            3 => {
                let node_id = ExpressionNodeId::new(unstasher.u64()? as _);
                let vec = unstasher.array_of_objects_vec()?;
                InternalASTNodeValue::Function(node_id, vec)
            }
            _ => panic!(),
        };

        Ok(InternalASTNode {
            value,
            self_rect: Cell::new(egui::Rect::NOTHING),
        })
    }
}
