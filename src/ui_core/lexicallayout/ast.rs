use std::cell::Cell;

use eframe::egui;

use crate::core::number::{
    numbergraph::NumberGraphInputId, numbergraphdata::NumberTarget, numbersource::NumberSourceId,
};

#[derive(Clone)]
pub(super) struct ASTPath {
    steps: Vec<usize>,
}

impl ASTPath {
    pub(super) fn new(steps: Vec<usize>) -> ASTPath {
        ASTPath { steps }
    }

    pub(super) fn at_beginning(&self) -> bool {
        self.steps.is_empty()
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

    pub(super) fn go_out(&mut self) {
        self.steps.pop();
    }

    pub(super) fn clear(&mut self) {
        self.steps.clear();
    }
}

pub(crate) struct VariableDefinition {
    name: String,
    value: ASTNode,
}

impl VariableDefinition {
    pub(super) fn new(name: String, value: ASTNode) -> VariableDefinition {
        VariableDefinition { name, value }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn value(&self) -> &ASTNode {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut ASTNode {
        &mut self.value
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ASTRoot<'a> {
    VariableDefinition(&'a VariableDefinition),
    FinalExpression,
}

#[derive(Copy, Clone)]
pub(crate) enum ASTNodeParent<'a> {
    VariableDefinition(&'a VariableDefinition),
    FinalExpression,
    InternalNode(&'a InternalASTNode, usize),
}

#[derive(Clone, Copy)]
pub(crate) enum ASTPathBuilder<'a> {
    Root(ASTRoot<'a>),
    ChildOf(&'a ASTPathBuilder<'a>, &'a InternalASTNode, usize),
}

impl<'a> ASTPathBuilder<'a> {
    pub(super) fn parent_node(&self) -> ASTNodeParent {
        match self {
            ASTPathBuilder::Root(ASTRoot::VariableDefinition(v)) => {
                ASTNodeParent::VariableDefinition(v)
            }
            ASTPathBuilder::Root(ASTRoot::FinalExpression) => ASTNodeParent::FinalExpression,
            ASTPathBuilder::ChildOf(_, n, i) => ASTNodeParent::InternalNode(n, *i),
        }
    }

    pub(super) fn push(
        &'a self,
        parent: &'a InternalASTNode,
        child_index: usize,
    ) -> ASTPathBuilder<'a> {
        ASTPathBuilder::ChildOf(self, parent, child_index)
    }

    pub(super) fn build(&self) -> ASTPath {
        fn helper(builder: &ASTPathBuilder, vec: &mut Vec<usize>) {
            if let ASTPathBuilder::ChildOf(parent_path, _parent_node, child_index) = builder {
                helper(parent_path, vec);
                vec.push(*child_index);
            }
        }

        let mut steps = Vec::new();
        helper(self, &mut steps);
        ASTPath { steps }
    }

    pub(super) fn matches_path(&self, path: &ASTPath) -> bool {
        fn helper(builder: &ASTPathBuilder, steps: &[usize]) -> bool {
            match builder {
                ASTPathBuilder::Root(_) => steps.is_empty(),
                ASTPathBuilder::ChildOf(parent_path, _parent_node, child_index) => {
                    let Some((last_step, other_steps)) = steps.split_last() else {
                        return false;
                    };
                    if last_step != child_index {
                        return false;
                    }
                    helper(parent_path, other_steps)
                }
            }
        }

        helper(self, &path.steps)
    }
}

pub(super) enum ASTNodeValue {
    Empty,
    Internal(Box<InternalASTNode>),
    Variable(String),
    GraphInput(NumberGraphInputId),
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

    pub(super) fn direct_target(&self) -> Option<NumberTarget> {
        match &self.value {
            ASTNodeValue::Empty => None,
            ASTNodeValue::Internal(node) => Some(node.number_source_id().into()),
            ASTNodeValue::Variable(_) => None,
            ASTNodeValue::GraphInput(giid) => Some((*giid).into()),
        }
    }

    pub(super) fn internal_node(&self) -> Option<&InternalASTNode> {
        match &self.value {
            ASTNodeValue::Internal(n) => Some(&*n),
            _ => None,
        }
    }

    pub(super) fn num_children(&self) -> usize {
        self.internal_node()
            .and_then(|n| Some(n.num_children()))
            .unwrap_or(0)
    }

    pub(super) fn rect(&self) -> egui::Rect {
        self.rect.get()
    }

    pub(super) fn set_rect(&self, rect: egui::Rect) {
        self.rect.set(rect);
    }

    fn is_over(&self, p: egui::Pos2) -> bool {
        self.rect().contains(p)
    }

    pub(super) fn is_directly_over(&self, p: egui::Pos2) -> bool {
        if !self.is_over(p) {
            return false;
        }
        if let ASTNodeValue::Internal(n) = &self.value {
            !(n.over_self(p) || n.over_children(p))
        } else {
            false
        }
    }

    pub(super) fn get_along_path(&self, path: &[usize]) -> &ASTNode {
        if path.is_empty() {
            self
        } else {
            let ASTNodeValue::Internal(node) = &self.value else {
                panic!()
            };
            node.get_along_path(path)
        }
    }

    pub(super) fn set_along_path(&mut self, path: &[usize], value: ASTNode) {
        if path.is_empty() {
            *self = value;
        } else {
            let ASTNodeValue::Internal(node) = &mut self.value else {
                panic!();
            };
            node.set_along_path(path, value);
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

    pub(super) fn is_empty(&self) -> bool {
        if let ASTNodeValue::Empty = &self.value {
            true
        } else {
            false
        }
    }
}

pub(super) enum InternalASTNodeValue {
    Prefix(NumberSourceId, ASTNode),
    Infix(ASTNode, NumberSourceId, ASTNode),
    Postfix(ASTNode, NumberSourceId),
    Function(NumberSourceId, Vec<ASTNode>),
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

    pub(super) fn number_source_id(&self) -> NumberSourceId {
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

    pub(super) fn self_rect(&self) -> egui::Rect {
        self.self_rect.get()
    }

    pub(super) fn set_self_rect(&self, rect: egui::Rect) {
        self.self_rect.set(rect);
    }

    pub(crate) fn over_self(&self, p: egui::Pos2) -> bool {
        self.self_rect().contains(p)
    }

    pub(super) fn over_children(&self, p: egui::Pos2) -> bool {
        match &self.value {
            InternalASTNodeValue::Prefix(_, c) => c.is_over(p),
            InternalASTNodeValue::Infix(c1, _, c2) => c1.is_over(p) || c2.is_over(p),
            InternalASTNodeValue::Postfix(c, _) => c.is_over(p),
            InternalASTNodeValue::Function(_, cs) => cs.iter().any(|c| c.is_over(p)),
        }
    }

    pub(super) fn get_along_path(&self, path: &[usize]) -> &ASTNode {
        let Some((next_step, rest_of_path)) = path.split_first() else {
            panic!("Empty paths can only be passed to ASTNode, not InternalASTNode");
        };
        let child_node = match (next_step, &self.value) {
            (0, InternalASTNodeValue::Prefix(_, c)) => c,
            (0, InternalASTNodeValue::Infix(c, _, _)) => c,
            (1, InternalASTNodeValue::Infix(_, _, c)) => c,
            (0, InternalASTNodeValue::Postfix(c, _)) => c,
            (i, InternalASTNodeValue::Function(_, cs)) => &cs[*i],
            _ => panic!("Invalid child index"),
        };
        child_node.get_along_path(rest_of_path)
    }

    fn set_along_path(&mut self, path: &[usize], value: ASTNode) {
        let Some((next_step, rest_of_path)) = path.split_first() else {
            panic!("Empty paths can only be passed to ASTNode, not InternalASTNode");
        };
        let child_node = match (next_step, &mut self.value) {
            (0, InternalASTNodeValue::Prefix(_, c)) => c,
            (0, InternalASTNodeValue::Infix(c, _, _)) => c,
            (1, InternalASTNodeValue::Infix(_, _, c)) => c,
            (0, InternalASTNodeValue::Postfix(c, _)) => c,
            (i, InternalASTNodeValue::Function(_, cs)) => &mut cs[*i],
            _ => panic!("Invalid child index"),
        };
        child_node.set_along_path(rest_of_path, value);
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
}
