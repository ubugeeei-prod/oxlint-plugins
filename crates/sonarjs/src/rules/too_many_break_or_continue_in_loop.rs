//! Rule `too-many-break-or-continue-in-loop` (SonarJS key S135).
//!
//! Clean-room port. A loop body should not contain more than one `break` or
//! `continue` statement that targets that loop. Having two or more jumps
//! escaping or restarting the same loop makes control flow hard to follow.
//!
//! The limit is fixed at one (i.e. two or more jumps trigger the rule). There
//! is no configurable threshold. The diagnostic is reported once per offending
//! loop, at the loop node's own span.
//!
//! Loops covered: `for`, `for…in`, `for…of`, `while`, `do…while`.
//!
//! Jump-targeting semantics:
//! - Unlabeled `break` targets the innermost enclosing loop **or** switch.
//!   A `break` inside a nested switch targets the switch (not the outer loop)
//!   and therefore does NOT count toward the outer loop.
//! - Unlabeled `continue` always targets the nearest enclosing **loop**
//!   (skips any intervening switch frames), so a `continue` inside a nested
//!   switch still counts for the enclosing loop.
//! - Labeled `break L` / `continue L` counts for a loop only if that loop
//!   carries the label `L`. A labeled break whose target is a block or a
//!   different construct counts for nothing.
//!
//! Behaviour is reproduced from the public RSPEC S135 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_span::Span;

use crate::scanner::{BreakableFrame, BreakableKind, Scanner};

pub(crate) const RULE_NAME: &str = "too-many-break-or-continue-in-loop";

impl<'a> Scanner<'a> {
    /// Pushes a new loop frame onto the breakable stack. Called on entry to
    /// every loop statement (`for`, `for…in`, `for…of`, `while`, `do…while`).
    pub(crate) fn enter_breakable_loop(&mut self, span: Span, label: Option<&'a str>) {
        self.breakable_stack.push(BreakableFrame {
            kind: BreakableKind::Loop,
            label,
            jump_count: 0,
            span,
        });
    }

    /// Pops the top loop frame and reports if the accumulated jump count
    /// exceeds one. Called on exit from every loop statement.
    pub(crate) fn leave_breakable_loop(&mut self) {
        let Some(frame) = self.breakable_stack.pop() else {
            return;
        };
        if frame.jump_count > 1 {
            self.report(RULE_NAME, "tooManyBreakContinue", frame.span);
        }
    }

    /// Pushes a new switch frame onto the breakable stack. Called on entry to
    /// every `switch` statement. Switch frames absorb unlabeled `break`
    /// statements so they do not propagate to an enclosing loop.
    pub(crate) fn enter_breakable_switch(&mut self, span: Span, label: Option<&'a str>) {
        self.breakable_stack.push(BreakableFrame {
            kind: BreakableKind::Switch,
            label,
            jump_count: 0,
            span,
        });
    }

    /// Pops the top switch frame. No diagnostic is emitted for switch frames.
    pub(crate) fn leave_breakable_switch(&mut self) {
        self.breakable_stack.pop();
    }

    /// Called for each `break` statement encountered during traversal.
    ///
    /// - Unlabeled: targets the innermost frame (loop or switch). If that
    ///   frame is a Loop, increment its jump count; if Switch, do nothing
    ///   (the break targets the switch, not any outer loop).
    /// - Labeled: search from the top of the stack for the first frame whose
    ///   label matches. If found and it is a Loop frame, increment that
    ///   frame's count.
    pub(crate) fn handle_break_jump(&mut self, label: Option<&'a str>) {
        match label {
            None => match self.breakable_stack.last_mut() {
                Some(frame) if matches!(frame.kind, BreakableKind::Loop) => {
                    frame.jump_count += 1;
                }
                _ => {}
            },
            Some(name) => {
                for frame in self.breakable_stack.iter_mut().rev() {
                    if frame.label == Some(name) {
                        if matches!(frame.kind, BreakableKind::Loop) {
                            frame.jump_count += 1;
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Called for each `continue` statement encountered during traversal.
    ///
    /// - Unlabeled: scan from the top of the stack, skip Switch frames,
    ///   and increment the count on the first Loop frame found.
    /// - Labeled: same as labeled `break` — search for the matching label
    ///   and increment if it is a Loop frame.
    pub(crate) fn handle_continue_jump(&mut self, label: Option<&'a str>) {
        match label {
            None => {
                for frame in self.breakable_stack.iter_mut().rev() {
                    if matches!(frame.kind, BreakableKind::Loop) {
                        frame.jump_count += 1;
                        break;
                    }
                }
            }
            Some(name) => {
                for frame in self.breakable_stack.iter_mut().rev() {
                    if frame.label == Some(name) {
                        if matches!(frame.kind, BreakableKind::Loop) {
                            frame.jump_count += 1;
                        }
                        break;
                    }
                }
            }
        }
    }
}
