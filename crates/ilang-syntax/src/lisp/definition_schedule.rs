//! Transactional retry scheduling for item-phase definition queries.
use crate::span::Span;
use std::collections::VecDeque;

/// Normal attempts wait for other item runs; fallback attempts resolve as if
/// the remaining runs cannot add a definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Fallback,
}

/// The result of one fresh macro run. A deferred or failed run contributes
/// nothing. Only answers made in fallback mode require a final audit.
pub enum Attempt<Product, Query, Answer, Error> {
    Complete {
        product: Product,
        answers: Vec<(Query, Answer)>,
    },
    Deferred {
        query: Query,
        span: Span,
    },
    Failed(Error),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Problem<Call, Query, Answer, Error> {
    Failed {
        call: Call,
        error: Error,
    },
    Waiting {
        call: Call,
        query: Query,
        span: Span,
    },
    Changed {
        call: Call,
        query: Query,
        before: Answer,
        after: Option<Answer>,
    },
}

/// Expand item calls until they have all committed, failed, or form a waiting
/// cycle. `attempt` receives only shared state, so a deferred run cannot leak
/// contributions. `commit` atomically installs a successful result and may
/// return newly introduced calls, which are processed before later siblings.
/// Every invocation of `attempt`, including a retry, must be a fresh macro run.
/// Final resolution rechecks every answer made during fallback.
pub fn schedule<State, Call, Query, Answer, Product, Error>(
    state: &mut State,
    calls: impl IntoIterator<Item = Call>,
    mut attempt: impl FnMut(&State, &Call, Mode) -> Attempt<Product, Query, Answer, Error>,
    mut commit: impl FnMut(&mut State, &Call, Product) -> Vec<Call>,
    mut final_answer: impl FnMut(&State, &Query) -> Option<Answer>,
) -> Vec<Problem<Call, Query, Answer, Error>>
where
    Call: Clone,
    Answer: PartialEq,
{
    struct Pending<C, Q> {
        call: C,
        waiting: Option<(Q, Span)>,
    }
    let mut queue: VecDeque<_> = calls
        .into_iter()
        .map(|call| Pending {
            call,
            waiting: None,
        })
        .collect();
    let mut problems = Vec::new();
    let mut audits = Vec::new();
    let mut mode = Mode::Normal;
    let mut deferred = 0;
    while let Some(mut pending) = queue.pop_front() {
        match attempt(state, &pending.call, mode) {
            Attempt::Complete { product, answers } => {
                if mode == Mode::Fallback {
                    audits.extend(
                        answers
                            .into_iter()
                            .map(|(query, answer)| (pending.call.clone(), query, answer)),
                    );
                }
                for call in commit(state, &pending.call, product).into_iter().rev() {
                    queue.push_front(Pending {
                        call,
                        waiting: None,
                    });
                }
                mode = Mode::Normal;
                deferred = 0;
            }
            Attempt::Failed(error) => {
                problems.push(Problem::Failed {
                    call: pending.call,
                    error,
                });
                mode = Mode::Normal;
                deferred = 0;
            }
            Attempt::Deferred { query, span } => {
                pending.waiting = Some((query, span));
                queue.push_back(pending);
                deferred += 1;
                if deferred == queue.len() {
                    if mode == Mode::Normal {
                        mode = Mode::Fallback;
                        deferred = 0;
                    } else {
                        for pending in queue.drain(..) {
                            let (query, span) = pending
                                .waiting
                                .expect("each pending run was tried in fallback");
                            problems.push(Problem::Waiting {
                                call: pending.call,
                                query,
                                span,
                            });
                        }
                    }
                }
            }
        }
    }
    for (call, query, before) in audits {
        let after = final_answer(state, &query);
        if after.as_ref() != Some(&before) {
            problems.push(Problem::Changed {
                call,
                query,
                before,
                after,
            });
        }
    }
    problems
}
