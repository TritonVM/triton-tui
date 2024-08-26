use arbitrary::Arbitrary;
use color_eyre::eyre::Result;
use itertools::Itertools;
use ratatui::prelude::*;
use ratatui::style::Styled;
use ratatui::widgets::block::Position;
use ratatui::widgets::block::Title;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
use triton_vm::instruction::*;
use triton_vm::op_stack::NUM_OP_STACK_REGISTERS;
use triton_vm::prelude::Program;
use triton_vm::prelude::Tip5;

use crate::action::Action;
use crate::action::Toggle;
use crate::element_type_hint::ElementTypeHint;
use crate::triton_vm_state::TritonVMState;

use super::Component;
use super::Frame;

#[derive(Debug, Clone, Arbitrary)]
pub(crate) struct Home {
    type_hints: bool,
    call_stack: bool,
    sponge: bool,
    inputs: bool,

    /// Lazily pre-rendered program. Reduces rendering time for long programs.
    rendered_program: Option<Vec<ProgramLine>>,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            type_hints: true,
            call_stack: true,
            sponge: false,
            inputs: true,
            rendered_program: None,
        }
    }
}

impl Home {
    fn render_program(program: &Program) -> Vec<ProgramLine> {
        let mut address = 0;
        let mut rendered_program = vec![];
        let mut has_breakpoint = false;
        for instruction in program.labelled_instructions() {
            let line = match instruction {
                LabelledInstruction::TypeHint(_) => continue,
                LabelledInstruction::Breakpoint => {
                    has_breakpoint = true;
                    continue;
                }
                LabelledInstruction::Label(label) => ProgramLine::Label(label),
                LabelledInstruction::Instruction(instruction) => {
                    let size = instruction.size();
                    let line = ProgramLine::Instruction {
                        address,
                        has_breakpoint,
                        instruction,
                    };
                    address += size;
                    has_breakpoint = false;
                    line
                }
            };
            rendered_program.push(line);
        }
        rendered_program
    }

    fn address_render_width(program: &Program) -> usize {
        let max_address = program.len_bwords();
        max_address.to_string().len()
    }

    fn toggle_widget(&mut self, toggle: Toggle) {
        match toggle {
            Toggle::All => self.toggle_all_widgets(),
            Toggle::TypeHint => self.type_hints = !self.type_hints,
            Toggle::CallStack => self.call_stack = !self.call_stack,
            Toggle::SpongeState => self.sponge = !self.sponge,
            Toggle::Input => self.inputs = !self.inputs,
            Toggle::BlockAddress => (),
        };
    }

    fn toggle_all_widgets(&mut self) {
        let any_widget_is_shown = self.all_widget_visibilities().into_iter().any(|v| v);
        self.set_all_widgets_visibility_to(!any_widget_is_shown);
    }

    fn all_widget_visibilities(&self) -> [bool; 4] {
        [self.type_hints, self.call_stack, self.sponge, self.inputs]
    }

    fn set_all_widgets_visibility_to(&mut self, visibility: bool) {
        self.type_hints = visibility;
        self.call_stack = visibility;
        self.sponge = visibility;
        self.inputs = visibility;
    }

    fn distribute_area_for_widgets(&self, state: &TritonVMState, area: Rect) -> WidgetAreas {
        let public_input_height = if self.maybe_render_public_input(state).is_some() {
            Constraint::Length(2)
        } else {
            Constraint::Length(0)
        };
        let secret_input_height = if self.maybe_render_secret_input(state).is_some() {
            Constraint::Length(2)
        } else {
            Constraint::Length(0)
        };
        let message_box_height = Constraint::Length(2);
        let constraints = [
            Constraint::Fill(1),
            public_input_height,
            secret_input_height,
            message_box_height,
        ];
        let [state_area, public_input, secret_input, message_box] =
            Layout::vertical(constraints).areas(area);

        let op_stack_widget_width = Constraint::Length(30);
        let remaining_width = Constraint::Fill(1);
        let sponge_state_width = if self.sponge {
            Constraint::Length(32)
        } else {
            Constraint::Length(1)
        };
        let [op_stack, remaining_area, sponge] =
            Layout::horizontal([op_stack_widget_width, remaining_width, sponge_state_width])
                .areas(state_area);

        let show = Constraint::Fill(1);
        let hide = Constraint::Length(0);
        let maybe_show = |is_visible| if is_visible { show } else { hide };
        let [type_hint, program, call_stack] = Layout::horizontal([
            maybe_show(self.type_hints),
            show,
            maybe_show(self.call_stack),
        ])
        .areas(remaining_area);

        WidgetAreas {
            op_stack,
            type_hint,
            program,
            call_stack,
            sponge,
            public_input,
            secret_input,
            message_box,
        }
    }

    fn render_op_stack_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        let op_stack = &render_info.state.vm_state.op_stack.stack;
        let render_area = render_info.areas.op_stack;

        let stack_size = op_stack.len();
        let title = format!(" Stack (size: {stack_size:>4}) ");
        let title = Title::from(title).alignment(Alignment::Left);

        let border_set = symbols::border::Set {
            bottom_left: symbols::line::ROUNDED.vertical_right,
            ..symbols::border::ROUNDED
        };
        let block = Block::default()
            .padding(Padding::new(1, 1, 1, 0))
            .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
            .border_set(border_set)
            .title(title);

        let num_available_lines = block.inner(render_area).height as usize;
        let num_padding_lines = num_available_lines.saturating_sub(stack_size);
        let mut text = vec![Line::from(""); num_padding_lines];
        for (i, st) in op_stack.iter().rev().enumerate() {
            let stack_index_style = match i {
                i if i < NUM_OP_STACK_REGISTERS => Style::new().bold(),
                _ => Style::new().dim(),
            };
            let stack_index = Span::from(format!("{i:>3}")).set_style(stack_index_style);
            let separator = Span::from("  ");
            let stack_element = Span::from(format!("{st}"));
            text.push(stack_index + separator + stack_element);
        }
        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
        frame.render_widget(paragraph, render_area);
    }

    fn render_type_hint_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        if !self.type_hints {
            return;
        }
        let block = Block::default()
            .padding(Padding::new(0, 1, 1, 0))
            .borders(Borders::TOP | Borders::BOTTOM);
        let render_area = render_info.areas.type_hint;
        let type_hints = &render_info.state.type_hints.stack;

        let num_available_lines = block.inner(render_area).height as usize;
        let num_padding_lines = num_available_lines.saturating_sub(type_hints.len());
        let mut text = vec![Line::from(""); num_padding_lines];

        let highest_hint = type_hints.last().cloned().flatten();
        let lowest_hint = type_hints.first().cloned().flatten();

        text.push(ElementTypeHint::render(&highest_hint).into());
        for (hint_0, hint_1, hint_2) in type_hints.iter().rev().tuple_windows() {
            if ElementTypeHint::is_continuous_sequence(&[hint_0, hint_1, hint_2]) {
                text.push("⋅".dim().into());
            } else {
                text.push(ElementTypeHint::render(hint_1).into());
            }
        }
        text.push(ElementTypeHint::render(&lowest_hint).into());

        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
        frame.render_widget(paragraph, render_area);
    }

    fn render_program_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        let state = &render_info.state;
        let cycle_count = state.vm_state.cycle_count;
        let title = format!(" Program (cycle: {cycle_count:>5}) ");
        let title = Title::from(title).alignment(Alignment::Left);

        let border_set = symbols::border::Set {
            top_left: symbols::line::ROUNDED.horizontal_down,
            bottom_left: symbols::line::ROUNDED.horizontal_up,
            ..symbols::border::ROUNDED
        };
        let block = Block::default()
            .padding(Padding::new(1, 1, 1, 0))
            .title(title)
            .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
            .border_set(border_set);

        let render_area = render_info.areas.program;
        let num_lines_to_render = usize::from(block.inner(render_area).height);
        let ip = state.vm_state.instruction_pointer;
        let Some(ref program) = self.rendered_program else {
            let err = Paragraph::new("\nRendering program…".yellow()).centered();
            frame.render_widget(err, render_area);
            return;
        };
        let Some(idx_of_line_with_ip) = Self::line_index_of_address(program, ip) else {
            let err = Paragraph::new(format!("\nNo instruction at address {ip}!").red()).centered();
            frame.render_widget(err, render_area);
            return;
        };
        let idx_of_first_line = idx_of_line_with_ip
            .saturating_add(num_lines_to_render / 2)
            .min(program.len())
            .saturating_sub(num_lines_to_render);

        let mut text = vec![];
        let address_width = Self::address_render_width(&state.program);
        for line in program
            .iter()
            .skip(idx_of_first_line)
            .take(num_lines_to_render)
        {
            let rendered_line = match line {
                ProgramLine::Label(label) => Line::from(format!(" {label}:")),
                &ProgramLine::Instruction {
                    address,
                    has_breakpoint,
                    ref instruction,
                } => {
                    let ip = if address == ip {
                        "→".bold()
                    } else {
                        " ".into()
                    };
                    let gutter = if has_breakpoint {
                        format!("{:>address_width$}  ", "🔴").into()
                    } else {
                        format!(" {address:>address_width$}  ").dim()
                    };
                    ip + gutter + Span::from(instruction.to_string())
                }
            };
            text.push(rendered_line);
        }

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, render_area);
    }

    /// Requires the [`ProgramLine`]s to be sorted by their address. Variants
    /// without an `address` field cannot be found this way.
    fn line_index_of_address(lines: &[ProgramLine], address_to_find: usize) -> Option<usize> {
        let indexed_instruction_lines = lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| match line {
                ProgramLine::Instruction { address, .. } => Some((idx, *address)),
                ProgramLine::Label(_) => None,
            })
            .collect_vec();
        let idx_idx = indexed_instruction_lines
            .binary_search_by_key(&address_to_find, |&(_, address)| address)
            .ok()?;
        let (idx, _) = indexed_instruction_lines[idx_idx];
        Some(idx)
    }

    fn render_call_stack_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        if !self.call_stack {
            return;
        }

        let state = &render_info.state;
        let jump_stack = &state.vm_state.jump_stack;

        let jump_stack_depth = jump_stack.len();
        let title = format!(" Calls (depth: {jump_stack_depth:>3}) ");
        let title = Title::from(title).alignment(Alignment::Left);

        let border_set = symbols::border::Set {
            top_left: symbols::line::ROUNDED.horizontal_down,
            bottom_left: symbols::line::ROUNDED.horizontal_up,
            ..symbols::border::ROUNDED
        };
        let block = Block::default()
            .padding(Padding::new(1, 1, 1, 0))
            .title(title)
            .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
            .border_set(border_set);
        let render_area = render_info.areas.call_stack;

        let num_available_lines = block.inner(render_area).height as usize;
        let num_padding_lines = num_available_lines.saturating_sub(jump_stack_depth);
        let mut text = vec![Line::from(""); num_padding_lines];

        let address_width = Self::address_render_width(&state.program);
        for (return_address, call_address) in jump_stack.iter().rev() {
            let return_address = return_address.value();
            let call_address = call_address.value();
            let addresses = Span::from(format!(
                "({return_address:>address_width$}, {call_address:>address_width$})"
            ));
            let separator = Span::from("  ");
            let label = Span::from(state.program.label_for_address(call_address));
            text.push(addresses + separator + label);
        }
        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
        frame.render_widget(paragraph, render_area);
    }

    fn render_sponge_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        let title = Title::from(" Sponge ");
        let border_set = symbols::border::Set {
            top_left: symbols::line::ROUNDED.horizontal_down,
            bottom_left: symbols::line::ROUNDED.horizontal_up,
            bottom_right: symbols::line::ROUNDED.vertical_left,
            ..symbols::border::ROUNDED
        };
        let borders = if self.sponge {
            Borders::ALL
        } else {
            Borders::TOP | Borders::RIGHT | Borders::BOTTOM
        };
        let block = Block::default()
            .borders(borders)
            .border_set(border_set)
            .title(title)
            .padding(Padding::new(1, 1, 1, 0));

        let render_area = render_info.areas.sponge;
        let Some(Tip5 { state: sponge }) = &render_info.state.vm_state.sponge else {
            let paragraph = Paragraph::new("").block(block);
            frame.render_widget(paragraph, render_area);
            return;
        };

        let num_available_lines = block.inner(render_area).height as usize;
        let num_padding_lines = num_available_lines.saturating_sub(sponge.len());
        let mut text = vec![Line::from(""); num_padding_lines];
        for (i, sp) in sponge.iter().enumerate() {
            let sponge_index = Span::from(format!("{i:>3}")).dim();
            let separator = Span::from("  ");
            let sponge_element = Span::from(format!("{sp}"));
            text.push(sponge_index + separator + sponge_element);
        }
        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
        frame.render_widget(paragraph, render_area);
    }

    fn render_public_input_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        let public_input = self
            .maybe_render_public_input(render_info.state)
            .unwrap_or_default();

        let border_set = symbols::border::Set {
            bottom_left: symbols::line::ROUNDED.vertical_right,
            bottom_right: symbols::line::ROUNDED.vertical_left,
            ..symbols::border::ROUNDED
        };
        let block = Block::default()
            .padding(Padding::horizontal(1))
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_set(border_set);
        let paragraph = Paragraph::new(public_input).block(block);
        frame.render_widget(paragraph, render_info.areas.public_input);
    }

    fn maybe_render_public_input(&self, state: &TritonVMState) -> Option<Line> {
        if state.vm_state.public_input.is_empty() || !self.inputs {
            return None;
        }
        let header = Span::from("Public input").bold();
        let colon = Span::from(": [");
        let input = state.vm_state.public_input.iter().join(", ");
        let input = Span::from(input);
        let footer = Span::from("]");
        Some(header + colon + input + footer)
    }

    fn render_secret_input_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        let secret_input = self
            .maybe_render_secret_input(render_info.state)
            .unwrap_or_default();

        let border_set = symbols::border::Set {
            bottom_left: symbols::line::ROUNDED.vertical_right,
            bottom_right: symbols::line::ROUNDED.vertical_left,
            ..symbols::border::ROUNDED
        };
        let block = Block::default()
            .padding(Padding::horizontal(1))
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_set(border_set);
        let paragraph = Paragraph::new(secret_input).block(block);
        frame.render_widget(paragraph, render_info.areas.secret_input);
    }

    fn maybe_render_secret_input(&self, state: &TritonVMState) -> Option<Line> {
        if state.vm_state.secret_individual_tokens.is_empty() || !self.inputs {
            return None;
        }
        let header = Span::from("Secret input").bold();
        let colon = Span::from(": [");
        let input = state.vm_state.secret_individual_tokens.iter().join(", ");
        let input = Span::from(input);
        let footer = Span::from("]");
        Some(header + colon + input + footer)
    }

    fn render_message_widget(&self, frame: &mut Frame<'_>, render_info: RenderInfo) {
        let message = self.message(render_info.state);
        let status = if render_info.state.vm_state.halting {
            Title::from(" HALT ".bold().green())
        } else {
            Title::default()
        };

        let block = Block::default()
            .padding(Padding::horizontal(1))
            .title(status)
            .title_position(Position::Bottom)
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_type(BorderType::Rounded);
        let paragraph = Paragraph::new(message).block(block);
        frame.render_widget(paragraph, render_info.areas.message_box);
    }

    fn message(&self, state: &TritonVMState) -> Line {
        self.maybe_render_error_message(state)
            .or_else(|| self.maybe_render_warning_message(state))
            .or_else(|| self.maybe_render_public_output(state))
            .unwrap_or_else(|| self.render_welcome_message())
    }

    fn maybe_render_error_message(&self, state: &TritonVMState) -> Option<Line> {
        let message = Span::from(state.error?.to_string());
        let error = "ERROR".bold().red();
        let colon = ": ".into();
        Some(error + colon + message)
    }

    fn maybe_render_warning_message(&self, state: &TritonVMState) -> Option<Line> {
        let message = Span::from(state.warning.as_ref()?.to_string());
        let warning = "WARNING".bold().yellow();
        let colon = ": ".into();
        Some(warning + colon + message)
    }

    fn maybe_render_public_output(&self, state: &TritonVMState) -> Option<Line> {
        if state.vm_state.public_output.is_empty() {
            return None;
        }
        let header = Span::from("Public output").bold();
        let colon = Span::from(": [");
        let output = state.vm_state.public_output.iter().join(", ");
        let output = Span::from(output);
        let footer = Span::from("]");
        Some(header + colon + output + footer)
    }

    fn render_welcome_message(&self) -> Line {
        let welcome = Span::from("Welcome to the Triton VM TUI! ");
        let help_hint = "Press `h` for help.".dim();
        welcome + help_hint
    }
}

impl Component for Home {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Toggle(toggle) => self.toggle_widget(toggle),
            Action::Reset => self.rendered_program = None,
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, state: &TritonVMState) -> Result<()> {
        self.rendered_program
            .get_or_insert_with(|| Self::render_program(&state.program));

        let render_info = RenderInfo {
            state,
            areas: self.distribute_area_for_widgets(state, frame.area()),
        };

        self.render_op_stack_widget(frame, render_info);
        self.render_type_hint_widget(frame, render_info);
        self.render_program_widget(frame, render_info);
        self.render_call_stack_widget(frame, render_info);
        self.render_sponge_widget(frame, render_info);
        self.render_public_input_widget(frame, render_info);
        self.render_secret_input_widget(frame, render_info);
        self.render_message_widget(frame, render_info);
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
struct RenderInfo<'s> {
    state: &'s TritonVMState,
    areas: WidgetAreas,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct WidgetAreas {
    op_stack: Rect,
    type_hint: Rect,
    program: Rect,
    call_stack: Rect,
    sponge: Rect,
    public_input: Rect,
    secret_input: Rect,
    message_box: Rect,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary)]
enum ProgramLine {
    Label(String),
    Instruction {
        address: usize,
        has_breakpoint: bool,
        instruction: AnInstruction<String>,
    },
}

#[cfg(test)]
mod tests {
    use proptest::prop_assert_eq;
    use proptest_arbitrary_interop::arb;
    use ratatui::backend::TestBackend;
    use test_strategy::proptest;
    use triton_vm::prelude::*;

    use crate::args::TuiArgs;

    use super::*;

    #[proptest]
    fn render_arbitrary_vm_state(
        #[strategy(arb())] mut home: Home,
        #[strategy(arb())] program: Program,
        #[strategy(arb())] mut vm_state: VMState,
    ) {
        vm_state.program.clone_from(&program.instructions);

        let mut complete_state = TritonVMState::new(&TuiArgs::default()).unwrap();
        complete_state.vm_state = vm_state;
        complete_state.program = program;

        let backend = TestBackend::new(150, 50);
        let mut terminal = Terminal::new(backend)?;
        terminal
            .draw(|f| home.draw(f, &complete_state).unwrap())
            .unwrap();
    }

    #[proptest]
    fn line_indices_of_empty_rendered_program_is_always_0(address: usize) {
        prop_assert_eq!(None, Home::line_index_of_address(&[], address));
    }

    #[proptest]
    fn searching_for_line_index_never_panics(#[strategy(arb())] program: Program, address: usize) {
        let lines = Home::render_program(&program);
        let _ = Home::line_index_of_address(&lines, address);
    }

    #[test]
    fn line_indices_in_rendered_program_with_only_instructions_can_be_found() {
        let instr = |address| ProgramLine::Instruction {
            address,
            has_breakpoint: false,
            instruction: AnInstruction::Nop,
        };
        let lines = vec![instr(0), instr(1), instr(2), instr(3)];

        assert_eq!(Some(0), Home::line_index_of_address(&lines, 0));
        assert_eq!(Some(1), Home::line_index_of_address(&lines, 1));
        assert_eq!(Some(2), Home::line_index_of_address(&lines, 2));
        assert_eq!(Some(3), Home::line_index_of_address(&lines, 3));
        assert_eq!(None, Home::line_index_of_address(&lines, 4));
        assert_eq!(None, Home::line_index_of_address(&lines, 5));
    }

    #[test]
    fn line_indices_in_rendered_program_starting_and_ending_with_labels_can_be_found() {
        let instruction = |address| ProgramLine::Instruction {
            address,
            has_breakpoint: false,
            instruction: AnInstruction::Nop,
        };
        let lines = vec![
            ProgramLine::Label("start".to_string()),
            instruction(0),
            instruction(1),
            ProgramLine::Label("middle".to_string()),
            instruction(2),
            instruction(3),
            ProgramLine::Label("end".to_string()),
        ];

        assert_eq!(Some(1), Home::line_index_of_address(&lines, 0));
        assert_eq!(Some(2), Home::line_index_of_address(&lines, 1));
        assert_eq!(Some(4), Home::line_index_of_address(&lines, 2));
        assert_eq!(Some(5), Home::line_index_of_address(&lines, 3));
        assert_eq!(None, Home::line_index_of_address(&lines, 4));
    }

    #[test]
    fn line_indices_in_rendered_program_with_many_labels_can_be_found() {
        let instruction = |address| ProgramLine::Instruction {
            address,
            has_breakpoint: false,
            instruction: AnInstruction::Nop,
        };
        let lines = vec![
            instruction(0),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            ProgramLine::Label("label".to_string()),
            instruction(2),
        ];

        assert_eq!(Some(0), Home::line_index_of_address(&lines, 0));
        assert_eq!(None, Home::line_index_of_address(&lines, 1));
        assert_eq!(Some(9), Home::line_index_of_address(&lines, 2));
        assert_eq!(None, Home::line_index_of_address(&lines, 3));
    }

    #[test]
    fn line_indices_in_rendered_program_containing_only_labels_can_never_be_found() {
        let lines = vec![
            ProgramLine::Label("start".to_string()),
            ProgramLine::Label("middle".to_string()),
            ProgramLine::Label("end".to_string()),
        ];

        assert_eq!(None, Home::line_index_of_address(&lines, 0));
        assert_eq!(None, Home::line_index_of_address(&lines, 1));
        assert_eq!(None, Home::line_index_of_address(&lines, 2));
    }
}
