use std::cell::Cell;

use eframe::egui;
use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::{
        graph::{
            graphobject::{ObjectType, WithObjectType},
            objectfactory::ObjectFactory,
        },
        number::{
            numbergraph::{NumberGraph, NumberGraphInputId},
            numbergraphdata::NumberTarget,
            numbergraphtopology::NumberGraphTopology,
            numbersource::NumberSourceId,
        },
        uniqueid::UniqueId,
    },
    objects::functions::Constant,
};

use super::{
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{AnyNumberObjectUiData, NumberGraphUiState, NumberObjectUiStates},
    soundnumberinputui::NumberSummonValue,
    summon_widget::{SummonWidget, SummonWidgetState, SummonWidgetStateBuilder},
    ui_factory::UiFactory,
};

#[derive(Clone)]
pub(super) struct ASTPath {
    steps: Vec<usize>,
}

impl ASTPath {
    pub(super) fn new(steps: Vec<usize>) -> ASTPath {
        ASTPath { steps }
    }

    fn at_beginning(&self) -> bool {
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

    fn go_into(&mut self, index: usize) {
        self.steps.push(index);
    }

    fn go_out(&mut self) {
        self.steps.pop();
    }

    fn clear(&mut self) {
        self.steps.clear();
    }
}

#[derive(Copy, Clone)]
enum ASTRoot<'a> {
    VariableDefinition(&'a VariableDefinition),
    FinalExpression,
}

#[derive(Copy, Clone)]
enum ASTNodeParent<'a> {
    VariableDefinition(&'a VariableDefinition),
    FinalExpression,
    InternalNode(&'a InternalASTNode, usize),
}

#[derive(Clone, Copy)]
enum ASTPathBuilder<'a> {
    Root(ASTRoot<'a>),
    ChildOf(&'a ASTPathBuilder<'a>, &'a InternalASTNode, usize),
}

impl<'a> ASTPathBuilder<'a> {
    fn parent_node(&self) -> ASTNodeParent {
        match self {
            ASTPathBuilder::Root(ASTRoot::VariableDefinition(v)) => {
                ASTNodeParent::VariableDefinition(v)
            }
            ASTPathBuilder::Root(ASTRoot::FinalExpression) => ASTNodeParent::FinalExpression,
            ASTPathBuilder::ChildOf(_, n, i) => ASTNodeParent::InternalNode(n, *i),
        }
    }

    fn push(&'a self, parent: &'a InternalASTNode, child_index: usize) -> ASTPathBuilder<'a> {
        ASTPathBuilder::ChildOf(self, parent, child_index)
    }

    fn build(&self) -> ASTPath {
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

    fn matches_path(&self, path: &ASTPath) -> bool {
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

pub(super) struct ASTNode {
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

    fn direct_target(&self) -> Option<NumberTarget> {
        match &self.value {
            ASTNodeValue::Empty => None,
            ASTNodeValue::Internal(node) => Some(node.number_source_id().into()),
            ASTNodeValue::Variable(_) => None,
            ASTNodeValue::GraphInput(giid) => Some((*giid).into()),
        }
    }

    fn internal_node(&self) -> Option<&InternalASTNode> {
        match &self.value {
            ASTNodeValue::Internal(n) => Some(&*n),
            _ => None,
        }
    }

    fn num_children(&self) -> usize {
        self.internal_node()
            .and_then(|n| Some(n.num_children()))
            .unwrap_or(0)
    }

    fn num_nonempty_children(&self) -> usize {
        self.internal_node()
            .and_then(|n| Some(n.num_nonempty_children()))
            .unwrap_or(0)
    }

    fn rect(&self) -> egui::Rect {
        self.rect.get()
    }

    fn set_rect(&self, rect: egui::Rect) {
        self.rect.set(rect);
    }

    fn is_over(&self, p: egui::Pos2) -> bool {
        self.rect().contains(p)
    }

    fn is_directly_over(&self, p: egui::Pos2) -> bool {
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

    fn set_along_path(&mut self, path: &[usize], value: ASTNode) {
        if path.is_empty() {
            *self = value;
        } else {
            let ASTNodeValue::Internal(node) = &mut self.value else {
                panic!();
            };
            node.set_along_path(path, value);
        }
    }

    fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(&self, path: ASTPathBuilder, f: &mut F) {
        f(self, path);
        if let ASTNodeValue::Internal(node) = &self.value {
            node.visit(path, f);
        }
    }

    fn is_empty(&self) -> bool {
        if let ASTNodeValue::Empty = &self.value {
            true
        } else {
            false
        }
    }
}

impl Default for NumberSourceLayout {
    fn default() -> Self {
        NumberSourceLayout::Function
    }
}

impl Serializable for NumberSourceLayout {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.u8(match self {
            NumberSourceLayout::Prefix => 1,
            NumberSourceLayout::Infix => 2,
            NumberSourceLayout::Postfix => 3,
            NumberSourceLayout::Function => 4,
        });
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(match deserializer.u8()? {
            1 => NumberSourceLayout::Prefix,
            2 => NumberSourceLayout::Infix,
            3 => NumberSourceLayout::Postfix,
            4 => NumberSourceLayout::Function,
            _ => return Err(()),
        })
    }
}

#[derive(Copy, Clone)]
pub enum NumberSourceLayout {
    Prefix,
    Infix,
    Postfix,
    Function,
}

pub(super) struct LexicalLayoutFocus {
    cursor: LexicalLayoutCursor,
    summon_widget_state: Option<SummonWidgetState<NumberSummonValue>>,
}

impl LexicalLayoutFocus {
    pub(super) fn new() -> LexicalLayoutFocus {
        LexicalLayoutFocus {
            cursor: LexicalLayoutCursor::new(),
            summon_widget_state: None,
        }
    }

    pub(super) fn cursor(&self) -> &LexicalLayoutCursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut LexicalLayoutCursor {
        &mut self.cursor
    }

    pub(super) fn summon_widget_state_mut(
        &mut self,
    ) -> &mut Option<SummonWidgetState<NumberSummonValue>> {
        &mut self.summon_widget_state
    }
}

pub(super) enum InternalASTNodeValue {
    Prefix(NumberSourceId, ASTNode),
    Infix(ASTNode, NumberSourceId, ASTNode),
    Postfix(ASTNode, NumberSourceId),
    Function(NumberSourceId, Vec<ASTNode>),
}

pub(super) struct InternalASTNode {
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

    fn number_source_id(&self) -> NumberSourceId {
        match &self.value {
            InternalASTNodeValue::Prefix(id, _) => *id,
            InternalASTNodeValue::Infix(_, id, _) => *id,
            InternalASTNodeValue::Postfix(_, id) => *id,
            InternalASTNodeValue::Function(id, _) => *id,
        }
    }

    fn num_children(&self) -> usize {
        match &self.value {
            InternalASTNodeValue::Prefix(_, _) => 1,
            InternalASTNodeValue::Infix(_, _, _) => 2,
            InternalASTNodeValue::Postfix(_, _) => 1,
            InternalASTNodeValue::Function(_, c) => c.len(),
        }
    }

    fn num_nonempty_children(&self) -> usize {
        let count = |child: &ASTNode| {
            if child.is_empty() {
                0
            } else {
                1
            }
        };
        match &self.value {
            InternalASTNodeValue::Prefix(_, c) => count(c),
            InternalASTNodeValue::Infix(c1, _, c2) => count(c1) + count(c2),
            InternalASTNodeValue::Postfix(c, _) => count(c),
            InternalASTNodeValue::Function(_, cs) => cs.iter().map(count).sum(),
        }
    }

    fn self_rect(&self) -> egui::Rect {
        self.self_rect.get()
    }

    fn set_self_rect(&self, rect: egui::Rect) {
        self.self_rect.set(rect);
    }

    fn over_self(&self, p: egui::Pos2) -> bool {
        self.self_rect().contains(p)
    }

    fn over_children(&self, p: egui::Pos2) -> bool {
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

fn make_internal_node(
    number_source_id: NumberSourceId,
    ui_data: &AnyNumberObjectUiData,
    arguments: Vec<ASTNode>,
) -> InternalASTNode {
    let value = match ui_data.layout() {
        NumberSourceLayout::Prefix => {
            assert_eq!(arguments.len(), 1);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Prefix(number_source_id, args.next().unwrap())
        }
        NumberSourceLayout::Infix => {
            assert_eq!(arguments.len(), 2);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Infix(
                args.next().unwrap(),
                number_source_id,
                args.next().unwrap(),
            )
        }
        NumberSourceLayout::Postfix => {
            assert_eq!(arguments.len(), 1);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Postfix(args.next().unwrap(), number_source_id)
        }
        NumberSourceLayout::Function => InternalASTNodeValue::Function(number_source_id, arguments),
    };
    InternalASTNode::new(value)
}

pub(super) struct LexicalLayoutCursor {
    line: usize,
    path: ASTPath,
}

impl LexicalLayoutCursor {
    pub(super) fn new() -> LexicalLayoutCursor {
        LexicalLayoutCursor {
            line: 0,
            path: ASTPath::new(Vec::new()),
        }
    }
}

struct VariableDefinition {
    name: String,
    value: ASTNode,
}

fn algebraic_key(key: egui::Key, modifiers: egui::Modifiers) -> Option<char> {
    match key {
        egui::Key::Minus => {
            if !modifiers.shift {
                Some('-')
            } else {
                None
            }
        }
        egui::Key::PlusEquals => {
            if modifiers.shift {
                Some('+')
            } else {
                None
            }
        }
        egui::Key::Num0 => Some('0'),
        egui::Key::Num1 => Some('1'),
        egui::Key::Num2 => Some('2'),
        egui::Key::Num3 => Some('3'),
        egui::Key::Num4 => Some('4'),
        egui::Key::Num5 => Some('5'),
        egui::Key::Num6 => Some('6'),
        egui::Key::Num7 => Some('7'),
        egui::Key::Num8 => Some('8'),
        egui::Key::Num9 => Some('9'),
        egui::Key::A => Some('a'),
        egui::Key::B => Some('b'),
        egui::Key::C => Some('c'),
        egui::Key::D => Some('d'),
        egui::Key::E => Some('e'),
        egui::Key::F => Some('f'),
        egui::Key::G => Some('g'),
        egui::Key::H => Some('h'),
        egui::Key::I => Some('i'),
        egui::Key::J => Some('j'),
        egui::Key::K => Some('k'),
        egui::Key::L => Some('l'),
        egui::Key::M => Some('m'),
        egui::Key::N => Some('n'),
        egui::Key::O => Some('o'),
        egui::Key::P => Some('p'),
        egui::Key::Q => Some('q'),
        egui::Key::R => Some('r'),
        egui::Key::S => Some('s'),
        egui::Key::T => Some('t'),
        egui::Key::U => Some('u'),
        egui::Key::V => Some('v'),
        egui::Key::W => Some('w'),
        egui::Key::X => Some('x'),
        egui::Key::Y => Some('y'),
        egui::Key::Z => Some('z'),
        _ => None,
    }
}

pub(super) struct LexicalLayout {
    variable_definitions: Vec<VariableDefinition>,
    final_expression: ASTNode,
}

impl LexicalLayout {
    pub(super) fn generate(
        topo: &NumberGraphTopology,
        object_ui_states: &NumberObjectUiStates,
    ) -> LexicalLayout {
        let outputs = topo.graph_outputs();
        assert_eq!(outputs.len(), 1);
        let output = &topo.graph_outputs()[0];

        let mut variable_assignments: Vec<VariableDefinition> = Vec::new();

        fn visit_target(
            target: NumberTarget,
            variable_assignments: &mut Vec<VariableDefinition>,
            topo: &NumberGraphTopology,
            object_ui_states: &NumberObjectUiStates,
        ) -> ASTNode {
            let nsid = match target {
                NumberTarget::Source(nsid) => nsid,
                NumberTarget::GraphInput(giid) => {
                    return ASTNode::new(ASTNodeValue::GraphInput(giid))
                }
            };

            if let Some(existing_variable) = variable_assignments
                .iter()
                .find(|va| va.value.direct_target() == Some(target))
            {
                return ASTNode::new(ASTNodeValue::Variable(existing_variable.name.clone()));
            }

            let create_new_variable = topo.number_target_destinations(target).count() >= 2;

            let arguments: Vec<ASTNode> = topo
                .number_source(nsid)
                .unwrap()
                .number_inputs()
                .iter()
                .map(|niid| match topo.number_input(*niid).unwrap().target() {
                    Some(target) => {
                        visit_target(target, variable_assignments, topo, object_ui_states)
                    }
                    None => ASTNode::new(ASTNodeValue::Empty),
                })
                .collect();

            let node = make_internal_node(nsid, object_ui_states.get_object_data(nsid), arguments);

            if create_new_variable {
                let new_variable_name = format!("x{}", variable_assignments.len());
                variable_assignments.push(VariableDefinition {
                    name: new_variable_name.clone(),
                    value: ASTNode::new(ASTNodeValue::Internal(Box::new(node))),
                });
                ASTNode::new(ASTNodeValue::Variable(new_variable_name))
            } else {
                ASTNode::new(ASTNodeValue::Internal(Box::new(node)))
            }
        }

        let final_expression = match output.target() {
            Some(target) => visit_target(target, &mut variable_assignments, topo, object_ui_states),
            None => ASTNode::new(ASTNodeValue::Empty),
        };

        LexicalLayout {
            variable_definitions: variable_assignments,
            final_expression,
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        result_label: &str,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        mut focus: Option<&mut LexicalLayoutFocus>,
    ) {
        let variable_definitions = &self.variable_definitions;
        let num_variable_definitions = variable_definitions.len();
        let final_expression = &self.final_expression;

        let mut cursor = focus.as_mut().and_then(|f| Some(f.cursor_mut()));

        ui.vertical(|ui| {
            for (i, var_def) in variable_definitions.iter().enumerate() {
                let line_number = i;
                Self::show_line(
                    ui,
                    &var_def.value,
                    &mut cursor,
                    line_number,
                    |ui, cursor, node| {
                        ui.horizontal(|ui| {
                            // TODO: make this and other text pretty
                            ui.label(format!("{} = ", var_def.name));
                            Self::show_child_ast_node(
                                ui,
                                node,
                                graph_state,
                                ctx,
                                ASTPathBuilder::Root(ASTRoot::VariableDefinition(var_def)),
                                cursor,
                            );
                            ui.label(",");
                        });
                    },
                );
            }
            if num_variable_definitions > 0 {
                ui.separator();
            }
            let line_number = variable_definitions.len();
            Self::show_line(
                ui,
                final_expression,
                &mut cursor,
                line_number,
                |ui, cursor, node| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} = ", result_label));
                        Self::show_child_ast_node(
                            ui,
                            node,
                            graph_state,
                            ctx,
                            ASTPathBuilder::Root(ASTRoot::FinalExpression),
                            cursor,
                        );
                        ui.label(".");
                    });
                },
            );
        });

        if let Some(summon_widget_state) = focus
            .and_then(|f| Some(f.summon_widget_state_mut().as_mut()))
            .flatten()
        {
            let summon_widget = SummonWidget::new(summon_widget_state);
            ui.add(summon_widget);
            // TODO: ?
        }
    }

    fn show_line<F: FnOnce(&mut egui::Ui, &mut Option<ASTPath>, &ASTNode)>(
        ui: &mut egui::Ui,
        node: &ASTNode,
        cursor: &mut Option<&mut LexicalLayoutCursor>,
        line_number: usize,
        add_contents: F,
    ) {
        let mut cursor_path = if let Some(cursor) = cursor {
            if cursor.line == line_number {
                Some(cursor.path.clone())
            } else {
                None
            }
        } else {
            None
        };

        add_contents(ui, &mut cursor_path, node);

        if let Some(mut path) = cursor_path {
            if let Some(cursor) = cursor {
                let (pressed_left, pressed_right) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight),
                    )
                });

                if pressed_left {
                    path.go_left(node);
                }
                if pressed_right {
                    path.go_right(node);
                }

                **cursor = LexicalLayoutCursor {
                    line: line_number,
                    path,
                };
            }
        }
    }

    fn show_child_ast_node(
        ui: &mut egui::Ui,
        node: &ASTNode,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
    ) {
        let hovering = ui
            .input(|i| i.pointer.hover_pos())
            .and_then(|p| Some(node.is_directly_over(p)))
            .unwrap_or(false);
        Self::with_cursor(ui, path, cursor, hovering, |ui, cursor| {
            let rect = match &node.value {
                ASTNodeValue::Empty => {
                    // TODO: show cursor?
                    let r = ui.label("???");
                    r.rect
                }
                ASTNodeValue::Internal(n) => {
                    let r = Self::show_internal_node(ui, n, graph_state, ctx, path, cursor);
                    r.rect
                }
                ASTNodeValue::Variable(name) => {
                    ui.add(egui::Label::new(
                        egui::RichText::new(&*name)
                            .code()
                            .color(egui::Color32::WHITE),
                    ))
                    .rect
                }
                ASTNodeValue::GraphInput(giid) => {
                    let name = format!("input{}", giid.value());
                    let r = ui
                        .add(egui::Label::new(
                            egui::RichText::new(name).code().color(egui::Color32::WHITE),
                        ))
                        .rect;
                    r
                }
            };
            node.set_rect(rect);
        });
    }

    fn show_internal_node(
        ui: &mut egui::Ui,
        node: &InternalASTNode,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
    ) -> egui::Response {
        let styled_text = |ui: &mut egui::Ui, s: String| -> egui::Response {
            let text = egui::RichText::new(s).code().color(egui::Color32::WHITE);
            ui.add(egui::Label::new(text))
        };

        let ir = ui.horizontal_centered(|ui| {
            let hovering_over_self = ui
                .input(|i| i.pointer.hover_pos())
                .and_then(|p| Some(node.over_self(p)))
                .unwrap_or(false);
            let own_rect = match &node.value {
                InternalASTNodeValue::Prefix(nsid, expr) => {
                    let r = Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        graph_state,
                        ctx,
                        path.push(node, 0),
                        cursor,
                    );
                    r
                }
                InternalASTNodeValue::Infix(expr1, nsid, expr2) => {
                    Self::show_child_ast_node(
                        ui,
                        expr1,
                        graph_state,
                        ctx,
                        path.push(node, 0),
                        cursor,
                    );
                    let r = Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr2,
                        graph_state,
                        ctx,
                        path.push(node, 1),
                        cursor,
                    );
                    r
                }
                InternalASTNodeValue::Postfix(expr, nsid) => {
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        graph_state,
                        ctx,
                        path.push(node, 0),
                        cursor,
                    );
                    Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                    })
                }
                InternalASTNodeValue::Function(nsid, exprs) => {
                    if exprs.is_empty() {
                        Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                            Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                        })
                    } else {
                        let frame = egui::Frame::default()
                            .inner_margin(2.0)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(32)));
                        frame
                            .show(ui, |ui| {
                                let r = Self::with_cursor(
                                    ui,
                                    path,
                                    cursor,
                                    hovering_over_self,
                                    |ui, _| {
                                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                                    },
                                );
                                styled_text(ui, "(".to_string());
                                if let Some((last_expr, other_exprs)) = exprs.split_last() {
                                    for (i, expr) in other_exprs.iter().enumerate() {
                                        Self::show_child_ast_node(
                                            ui,
                                            expr,
                                            graph_state,
                                            ctx,
                                            path.push(node, i),
                                            cursor,
                                        );
                                        styled_text(ui, ",".to_string());
                                    }
                                    Self::show_child_ast_node(
                                        ui,
                                        last_expr,
                                        graph_state,
                                        ctx,
                                        path.push(node, other_exprs.len()),
                                        cursor,
                                    );
                                }
                                styled_text(ui, ")".to_string());
                                r
                            })
                            .inner
                    }
                }
            };

            node.set_self_rect(own_rect);
        });

        ir.response
    }

    fn show_number_source_ui(
        ui: &mut egui::Ui,
        id: NumberSourceId,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
    ) -> egui::Rect {
        let graph_object = ctx
            .topology()
            .number_source(id)
            .unwrap()
            .instance_arc()
            .as_graph_object();
        let object_type = graph_object.get_type();
        let object_ui = ctx.ui_factory().get_object_ui(object_type);
        let object_state = ctx.object_ui_states().get_object_data(id);
        ui.horizontal_centered(|ui| {
            object_ui.apply(&graph_object, &object_state, graph_state, ui, ctx);
        })
        .response
        .rect
    }

    fn flashing_highlight_color(ui: &mut egui::Ui) -> egui::Color32 {
        let t = ui.input(|i| i.time);
        let a = (((t - t.floor()) * 2.0 * std::f64::consts::TAU).sin() * 16.0 + 64.0) as u8;
        egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, a)
    }

    fn with_cursor<R, F: FnOnce(&mut egui::Ui, &mut Option<ASTPath>) -> R>(
        ui: &mut egui::Ui,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
        hovering: bool,
        add_contents: F,
    ) -> R {
        let highlight = cursor
            .as_ref()
            .and_then(|c| Some(path.matches_path(c)))
            .unwrap_or(false);
        let ret;
        {
            let color = if highlight {
                Self::flashing_highlight_color(ui)
            } else {
                // egui::Color32::TRANSPARENT
                egui::Color32::from_black_alpha(64)
            };
            let frame = egui::Frame::default()
                .inner_margin(2.0)
                .fill(color)
                .stroke(egui::Stroke::new(2.0, color));
            let r = frame.show(ui, |ui| add_contents(ui, cursor));
            ret = r.inner;

            let r = r.response.interact(egui::Sense::click_and_drag());

            if r.clicked() || r.dragged() {
                *cursor = Some(path.build());
            }
            // if r.hovered() {
            //     ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            // }

            let hover_amount = ui.ctx().animate_bool(r.id, hovering);
            if hover_amount > 0.0 {
                ui.painter().rect_stroke(
                    r.rect,
                    egui::Rounding::none(),
                    egui::Stroke::new(
                        2.0,
                        egui::Color32::from_white_alpha((hover_amount * 64.0) as u8),
                    ),
                );
            }
        }
        ret
    }

    pub(super) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        numbergraph: &mut NumberGraph,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
    ) {
        // TODO: consider filtering egui's InputState's vec of inputs
        // and consuming key presses from there

        {
            let cursor = focus.cursor_mut();
            let (pressed_up, pressed_down) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                )
            });
            if pressed_up {
                cursor.line = cursor.line.saturating_sub(1);
                cursor.path.clear();
            }
            if pressed_down {
                cursor.line = (cursor.line + 1).min(self.variable_definitions.len());
                cursor.path.clear();
            }

            let pressed_delete =
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));

            if pressed_delete {
                self.delete_from_numbergraph_at_cursor(focus.cursor_mut(), numbergraph);
            }
        }

        self.handle_summon_widget(
            ui,
            focus,
            numbergraph,
            object_factory,
            ui_factory,
            object_ui_states,
        );

        // TODO: create a summon widget similar to that used in flosion_ui.
        // start typing to gather a list of candidate number sources.
        // Ideally the experience should have minimal overhead typing,
        // e.g. typing something like "sin x + 2 * b" should result in sin(x + (2 * b))
        // with all intermediate values nicely built up. This might require knowing
        // operator precedence and doing funny things with the cursor but hopefully not.
        // Intermediates with cursor:
        // (no input yet)   -> _
        //                   ^
        // "sin "           -> sin(_)
        //                       ^
        // "x "             -> sin(x)
        //                         ^
        // "+ "             -> sin(x + _)           NOTE: so typing an infix operator nests the selected ast node
        //                             ^            as the left child and places the cursor on the right child?
        //
        // "2 "             -> sin(x + 2)
        //                             ^
        // "* "             -> sin(x + (2 * _))     NOTE: operator precedence could be applied here to determine
        //                                  ^       whether or not to place a node inside or around the parent(s)
        //
        // "b "             -> sin(x + (2 * b))
        //                                  ^
        //
        // left a bunch     -> sin(x + (2 * b))     NOTE: could press home also
        //                     ^
        // "^ ""            -> sin(x + (2 * b))^_
        //                                      ^
        // "1 "             -> sin(x + (2 * b))^1
        //                                      ^
        // "/"              -> sin(x + (2 * b))^(1/_)
        //                                         ^
        // "2 "             -> sin(x + (2 * b))^(1/2)
    }

    fn build_summon_widget(
        &self,
        position: egui::Pos2,
        ui_factory: &UiFactory<NumberGraphUi>,
    ) -> SummonWidgetState<NumberSummonValue> {
        let mut builder = SummonWidgetStateBuilder::new(position);
        for object_type in ui_factory.all_object_types() {
            builder.add_basic_name(
                object_type.name().to_string(),
                NumberSummonValue::NumberSourceType(object_type),
            );
        }

        // TODO: move this to the object ui after testing
        builder.add_pattern("constant".to_string(), |s| {
            // TODO: actually use the parsed value as part of initializing the constant
            // This should probably be done with a per-object/ui initialization type
            s.parse::<f32>()
                .ok()
                .and(Some(NumberSummonValue::NumberSourceType(Constant::TYPE)))
        });
        builder.build()
    }

    fn handle_summon_widget(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        numbergraph: &mut NumberGraph,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
    ) {
        let pressed_space_or_tab = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
        });

        let algebraic_keys_pressed = ui.input_mut(|input| {
            let mut out_chars = Vec::new();
            input.events = input
                .events
                .iter()
                .filter(|e| {
                    if let egui::Event::Key {
                        key,
                        pressed,
                        repeat: _,
                        modifiers,
                    } = e
                    {
                        if *pressed {
                            if let Some(ch) = algebraic_key(*key, *modifiers) {
                                out_chars.push(ch);
                                return false;
                            }
                        }
                    }
                    true
                })
                .cloned()
                .collect();
            out_chars
        });

        if focus.summon_widget_state_mut().is_none() {
            if pressed_space_or_tab || !algebraic_keys_pressed.is_empty() {
                //  open summon widget when space/tab is pressed
                let node_at_cursor = self.get_node_at_cursor(&focus.cursor());
                let mut widget_state =
                    self.build_summon_widget(node_at_cursor.rect().center_bottom(), ui_factory);
                let s = String::from_iter(algebraic_keys_pressed);
                widget_state.set_text(s);

                *focus.summon_widget_state_mut() = Some(widget_state);
            }
        }

        if let Some(summon_widget_state) = focus.summon_widget_state_mut() {
            if let Some(choice) = summon_widget_state.final_choice() {
                match choice {
                    NumberSummonValue::NumberSourceType(ns_type) => {
                        let (node, layout) = self
                            .create_new_number_source_from_type(
                                ns_type,
                                object_factory,
                                ui_factory,
                                object_ui_states,
                                numbergraph,
                            )
                            .unwrap();

                        let num_children = node.num_children();
                        self.insert_to_numbergraph_at_cursor(focus.cursor_mut(), node, numbergraph);

                        let cursor = focus.cursor_mut();
                        match layout {
                            NumberSourceLayout::Prefix => cursor.path.go_into(0),
                            NumberSourceLayout::Infix => cursor.path.go_into(0),
                            NumberSourceLayout::Postfix => cursor.path.go_into(0),
                            NumberSourceLayout::Function => {
                                if num_children > 0 {
                                    cursor.path.go_into(0);
                                }
                            }
                        }
                    }
                    NumberSummonValue::SoundNumberSource(snsid) => {
                        todo!("TODO: handle this OUTSIDE of lexical layout")
                    }
                };
                *focus.summon_widget_state_mut() = None;
            }
        }
    }

    fn create_new_number_source_from_type(
        &self,
        ns_type: ObjectType,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        numbergraph: &mut NumberGraph,
    ) -> Result<(ASTNode, NumberSourceLayout), String> {
        let new_object = object_factory.create_default(ns_type.name(), numbergraph);

        let new_object = match new_object {
            Ok(o) => o,
            Err(_) => {
                return Err(format!(
                    "Failed to create number object of type {}",
                    ns_type.name()
                ));
            }
        };

        let new_ui_state = ui_factory.create_default_state(&new_object);

        let num_inputs = numbergraph
            .topology()
            .number_source(new_object.id())
            .unwrap()
            .number_inputs()
            .len();
        let child_nodes: Vec<ASTNode> = (0..num_inputs)
            .map(|_| ASTNode::new(ASTNodeValue::Empty))
            .collect();
        let internal_node = make_internal_node(new_object.id(), &new_ui_state, child_nodes);
        let node = ASTNode::new(ASTNodeValue::Internal(Box::new(internal_node)));

        let layout = new_ui_state.layout();

        object_ui_states.set_object_data(new_object.id(), new_ui_state);

        Ok((node, layout))
    }

    fn get_cursor_root(&self, cursor: &LexicalLayoutCursor) -> Option<ASTRoot> {
        if cursor.path.at_beginning() {
            if cursor.line < self.variable_definitions.len() {
                Some(ASTRoot::VariableDefinition(
                    &self.variable_definitions[cursor.line],
                ))
            } else if cursor.line == self.variable_definitions.len() {
                Some(ASTRoot::FinalExpression)
            } else {
                panic!("Cursor out of range")
            }
        } else {
            None
        }
    }

    fn get_node_at_cursor(&self, cursor: &LexicalLayoutCursor) -> &ASTNode {
        if cursor.line < self.variable_definitions.len() {
            self.variable_definitions[cursor.line]
                .value
                .get_along_path(cursor.path.steps())
        } else if cursor.line == self.variable_definitions.len() {
            self.final_expression.get_along_path(cursor.path.steps())
        } else {
            panic!("Invalid line number")
        }
    }

    fn set_node_at_cursor(&mut self, cursor: &LexicalLayoutCursor, value: ASTNode) {
        if cursor.line < self.variable_definitions.len() {
            self.variable_definitions[cursor.line]
                .value
                .set_along_path(cursor.path.steps(), value);
        } else if cursor.line == self.variable_definitions.len() {
            self.final_expression
                .set_along_path(cursor.path.steps(), value);
        } else {
            panic!("Invalid line number")
        }
    }

    fn delete_from_numbergraph_at_cursor(
        &mut self,
        cursor: &LexicalLayoutCursor,
        numbergraph: &mut NumberGraph,
    ) {
        if self.get_node_at_cursor(cursor).is_empty() {
            return;
        }

        // If the cursor is pointing at a variable definition or the final expression,
        // disconnect those
        match self.get_cursor_root(cursor) {
            Some(ASTRoot::VariableDefinition(var_def)) => {
                self.disconnect_each_variable_use(&var_def.name, numbergraph);
            }
            Some(ASTRoot::FinalExpression) => {
                let graph_outputs = numbergraph.topology().graph_outputs();
                debug_assert_eq!(graph_outputs.len(), 1);
                let graph_output = graph_outputs.first().unwrap();
                debug_assert_eq!(self.final_expression.direct_target(), graph_output.target());
                numbergraph
                    .disconnect_graph_output(graph_output.id())
                    .unwrap();
            }
            None => (),
        }

        let node = self.get_node_at_cursor(cursor);
        if let Some(internal_node) = node.internal_node() {
            self.delete_internal_node_from_graph(internal_node, numbergraph);
        }
        self.set_node_at_cursor(cursor, ASTNode::new(ASTNodeValue::Empty));

        self.remove_unreferenced_graph_inputs(numbergraph);
    }

    fn insert_to_numbergraph_at_cursor(
        &mut self,
        cursor: &mut LexicalLayoutCursor,
        node: ASTNode,
        numbergraph: &mut NumberGraph,
    ) {
        debug_assert_eq!(node.num_nonempty_children(), 0);

        // TODO: allow inserting operators in-place
        self.delete_from_numbergraph_at_cursor(cursor, numbergraph);

        if let Some(target) = node.direct_target() {
            match self.get_cursor_root(cursor) {
                Some(ASTRoot::VariableDefinition(var_def)) => {
                    // if the cursor points to a variable definition, reconnect each use
                    self.connect_each_variable_use(&var_def.name, target, numbergraph);
                }
                Some(ASTRoot::FinalExpression) => {
                    // if the cursor points to the final expression, reconnect
                    // the graph output
                    let graph_outputs = numbergraph.topology().graph_outputs();
                    debug_assert_eq!(graph_outputs.len(), 1);
                    let graph_output = graph_outputs.first().unwrap();
                    debug_assert_eq!(self.final_expression.direct_target(), None);
                    numbergraph
                        .connect_graph_output(graph_output.id(), target)
                        .unwrap();
                }
                None => {
                    // if the cursor points to an ordinary internal node, reconnect
                    // just its parent
                    let mut cursor_to_parent = LexicalLayoutCursor {
                        line: cursor.line,
                        path: cursor.path.clone(),
                    };
                    cursor_to_parent.path.go_out();
                    let parent_node = self.get_node_at_cursor(&cursor_to_parent);
                    let ASTNodeValue::Internal(parent_node) = parent_node.value() else {
                        panic!()
                    };
                    let child_index = *cursor.path.steps().last().unwrap();
                    let parent_nsid = parent_node.number_source_id();
                    let parent_ns = numbergraph.topology().number_source(parent_nsid).unwrap();
                    let parent_inputs = parent_ns.number_inputs();
                    debug_assert_eq!(parent_inputs.len(), parent_node.num_children());
                    let input_id = parent_inputs[child_index];
                    numbergraph.connect_number_input(input_id, target).unwrap();
                }
            }
        }

        self.set_node_at_cursor(cursor, node);
    }

    fn delete_internal_node_from_graph(
        &self,
        node: &InternalASTNode,
        numbergraph: &mut NumberGraph,
    ) {
        let nsid = node.number_source_id();
        let mut dsts = numbergraph
            .topology()
            .number_target_destinations(NumberTarget::Source(nsid));
        let dst = dsts.next();
        // There should only be one thing connected to the number source at this point
        debug_assert!(dsts.next().is_none());
        std::mem::drop(dsts);
        if let Some(dst) = dst {
            numbergraph.disconnect_destination(dst).unwrap();
        };

        fn visit_node(node: &ASTNode, numbergraph: &mut NumberGraph) {
            if let Some(internal_node) = node.internal_node() {
                visitor_internal_node(internal_node, numbergraph);
            }
        }

        fn visitor_internal_node(node: &InternalASTNode, numbergraph: &mut NumberGraph) {
            let nsid = node.number_source_id();

            // Recursively delete any number sources corresponding to direct AST children
            match node.value() {
                InternalASTNodeValue::Prefix(_, c) => {
                    visit_node(c, numbergraph);
                }
                InternalASTNodeValue::Infix(c1, _, c2) => {
                    visit_node(c1, numbergraph);
                    visit_node(c2, numbergraph);
                }
                InternalASTNodeValue::Postfix(c, _) => {
                    visit_node(c, numbergraph);
                }
                InternalASTNodeValue::Function(_, cs) => {
                    for c in cs {
                        visit_node(c, numbergraph);
                    }
                }
            }

            // Delete the number source itself
            numbergraph.remove_number_source(nsid).unwrap();
        }

        visitor_internal_node(node, numbergraph);
    }

    fn disconnect_each_variable_use(&self, name: &str, numbergraph: &mut NumberGraph) {
        self.visit(|node, path| {
            let ASTNodeValue::Variable(var_name) = &node.value else {
                return;
            };
            if var_name != name {
                return;
            }
            match path.parent_node() {
                ASTNodeParent::VariableDefinition(var_def) => {
                    debug_assert_ne!(var_def.name, name);
                    // FUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUCK aliasing
                    panic!();
                }
                ASTNodeParent::FinalExpression => {
                    let outputs = numbergraph.topology().graph_outputs();
                    debug_assert_eq!(outputs.len(), 1);
                    let goid = outputs[0].id();
                    numbergraph.disconnect_graph_output(goid).unwrap();
                }
                ASTNodeParent::InternalNode(internal_node, child_index) => {
                    let nsid = internal_node.number_source_id();
                    let number_inputs = numbergraph
                        .topology()
                        .number_source(nsid)
                        .unwrap()
                        .number_inputs();
                    debug_assert_eq!(number_inputs.len(), internal_node.num_children());
                    let niid = number_inputs[child_index];
                    numbergraph.disconnect_number_input(niid).unwrap();
                }
            }
        });
    }

    fn connect_each_variable_use(
        &self,
        name: &str,
        target: NumberTarget,
        numbergraph: &mut NumberGraph,
    ) {
        self.visit(|node, path| {
            let ASTNodeValue::Variable(var_name) = &node.value else {
                return;
            };
            if var_name != name {
                return;
            }
            match path.parent_node() {
                ASTNodeParent::VariableDefinition(var_def) => {
                    debug_assert_ne!(var_def.name, name);
                    // FUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUCK aliasing
                    panic!();
                }
                ASTNodeParent::FinalExpression => {
                    let outputs = numbergraph.topology().graph_outputs();
                    debug_assert_eq!(outputs.len(), 1);
                    let goid = outputs[0].id();
                    numbergraph.connect_graph_output(goid, target).unwrap();
                }
                ASTNodeParent::InternalNode(internal_node, child_index) => {
                    let nsid = internal_node.number_source_id();
                    let number_inputs = numbergraph
                        .topology()
                        .number_source(nsid)
                        .unwrap()
                        .number_inputs();
                    debug_assert_eq!(number_inputs.len(), internal_node.num_children());
                    let niid = number_inputs[child_index];
                    numbergraph.connect_number_input(niid, target).unwrap();
                }
            }
        });
    }

    fn remove_unreferenced_graph_inputs(&self, numbergraph: &mut NumberGraph) {
        let mut referenced_graph_inputs = Vec::<NumberGraphInputId>::new();

        self.visit(|node, _path| {
            if let ASTNodeValue::GraphInput(giid) = node.value() {
                if !referenced_graph_inputs.contains(&giid) {
                    referenced_graph_inputs.push(*giid);
                }
            }
        });

        debug_assert!((|| {
            for giid in &referenced_graph_inputs {
                if !numbergraph.topology().graph_inputs().contains(giid) {
                    return false;
                }
            }
            true
        })());

        let all_graph_inputs = numbergraph.topology().graph_inputs().to_vec();
        for giid in all_graph_inputs {
            if !referenced_graph_inputs.contains(&giid) {
                debug_assert_eq!(
                    numbergraph
                        .topology()
                        .number_target_destinations(NumberTarget::GraphInput(giid))
                        .count(),
                    0
                );
                // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa
                // this leaves a dangling entry in the SoundNumberInputData's
                // mapping. BUT, LexicalLayout shouldn't care about the details
                // of SoundNumberInputData since it should work generically for
                // any numbergraph alone (to allow for later top-level number graphs)
                // Removing graph inputs is easy enough (just have the SoundNumberInputData
                // detect and remove its own surplus mapping entry) but adding a new
                // mapping entry is tricky since the SoundNumberInputData can't know
                // which SoundNumberSource it's supposed to be connected to afterwards.
                // Does this imply that some kind of hook/callback is needed to add
                // custom functionality when a number graph input is added via the
                // summon widget? In principle, the summon widget is a great place
                // for this kind of thing since 1) its entries already must differ
                // for sound number inputs in order to account for external sound
                // number sources and 2) this would allow for other functions to
                // be called when a summon widget entry is chosen which might prove
                // useful. For example, in top level number graphs, the same kind
                // of function could add a new named parameter
                // --------------
                // TODO:
                // - remove all references to SoundNumberInputId from LexicalLayout
                // - add a mechanism to LexicalLayout to define a custom summon
                //   widget entry with a custom callback function
                // - use that mechanism in SoundNumberInputUi to add sound number
                //   sources by name to the LexicalLayout's summon widget choices,
                //   make sure that results in new graph inputs being added and
                //   existing graph inputs being reused in agreement with the
                //   SoundNumberInputData's current mapping
                numbergraph.remove_graph_input(giid).unwrap();
            }
        }
    }

    fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(&self, mut f: F) {
        for vardef in &self.variable_definitions {
            vardef.value.visit(
                ASTPathBuilder::Root(ASTRoot::VariableDefinition(vardef)),
                &mut f,
            );
        }
        self.final_expression
            .visit(ASTPathBuilder::Root(ASTRoot::FinalExpression), &mut f);
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        // TODO: check whether anything was removed, update the layout somehow.
    }
}
