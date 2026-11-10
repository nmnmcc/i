# R2 public interface audit

The source is runtime.md §5.1's request table and §6.1's task table, also
normative spec §§9.9/9.11. `cargo test -p ilang-check --test library --locked`
checks the exact public names. The inferred output is produced by
`UPDATE=1 cargo test -p ilang-check --test types --locked -- library`, in
crates/ilang-check/tests/types/library.out. It includes private definitions;
those are not extra public exports.

The following lists the inferred public schemes, modulo bound-variable names,
parameter labels and the redundant intersections described below. A, B, E, F,
N, M and S are quantified binding variables, not new primitive types. All table
entries match the corresponding source-table row.

| i/request binding | Inferred scheme |
| --- | --- |
| open_root | `() -> Root ! Root` |
| close_root | `(Root) -> Nil` |
| open_owner | `() -> Owner ! Root` |
| close_owner | `(Owner) -> Nil` |
| count | `(Owner) -> Int` |
| count_root | `(Root) -> Int` |
| settled_owner | `(Owner) -> Bool` |
| settled_root | `(Root) -> Bool` |
| cancel_all | `(Owner) -> Nil` |
| cancel_all_root | `(Root) -> Nil` |
| wait_none | `(Owner) -> Nil ! Await` |
| wait_none_root | `(Root) -> Nil ! Await` |
| within | `(() -> A ? E ! N) -> A ? E ! Root | N - Owner` |
| ready | `(A) -> Request(A)` |
| ticket | `(Request(A)) -> Ticket` |
| state | `(Request(A)) -> RequestState(A)` |
| take | `(Request(A)) -> A` |
| settled | `(Ticket) -> Bool` |
| cancel | `(Ticket) -> Nil` |
| await | `(Request(A)) -> A ! Await` |
| await_owned | `(Request(A)) -> A ! Await` |
| wait_any | `(List(Ticket)) -> Nil` |
| poll | `() -> Nil` |

Root, Owner, Await and RequestState export the constructors in §5.1.
Request/Ticket and Key/Keyed remain primitive types, not module re-exports.

| i/task binding | Inferred scheme |
| --- | --- |
| top | `(random.Generator) -> Runner ! Runner` |
| run | `(() -> A ? E ! N) -> A ? E ! Runner | N - Scheduler - Runner - request.Await - request.Owner - request.Root` |
| test | `(random.Generator, List(List(Int)), () -> A ? E ! N) -> A ? E | ScheduleError ! Runner | N - Scheduler - Runner - request.Await - request.Owner - request.Root` |
| scope | `((Scope(E)) -> A ? E ! N) -> A ? E ! N | Scheduler` |
| local_scope | `((Scope(E)) -> A ? E ! N) -> A ? E ! N | Scheduler` |
| go | `(Scope(E), () -> A ? E ! N) -> Task(A,E) ! Scheduler | N - Scheduler - Runner - request.Await` |
| join | `(Task(A,E)) -> A ? E ! Scheduler` |
| yield | `() -> Nil ! Scheduler` |
| path | `() -> List(Int) ! Scheduler` |
| random | `() -> random.Generator ! Scheduler` |
| keep | `((S) -> A ? E ! N) -> { (S) -> A ? E ! Scheduler } ! Scheduler | N - Scheduler - Runner - request.Await` |
| channel | `() -> Channel(A) ! Scheduler` |
| local_channel | `() -> Channel(A) ! Scheduler` |
| send | `(Channel(A), A) -> Nil ! Scheduler` |
| receive | `(Channel(A)) -> A ! Scheduler` |
| send_event | `(Channel(A), A) -> Event(Nil,Never,Never)` |
| receive_event | `(Channel(A)) -> Event(A,Never,Never)` |
| join_event | `(Task(A,E)) -> Event(A,E,Never)` |
| always | `(A) -> Event(A,Never,Never)` |
| never | `() -> Event(Never,Never,Never)` |
| choose | `(List(Event(A,E,N))) -> Event(A,E,N)` |
| map | `(Event(A,E,N), (A) -> B ? F ! M) -> Event(B,E | F,N | M)` |
| sync | `(Event(A,E,N)) -> A ? E ! N | Scheduler` |
| share | `(Scope(E), A) -> Shared(A) ! Scheduler` |
| get | `(Shared(A)) -> A ! Scheduler` |
| both | `(() -> A ? E ! N, () -> B ? E ! N) -> (A,B) ? E ! Scheduler | N - Scheduler - Runner - request.Await` |
| race | `(() -> A ? E ! N, () -> A ? E ! N) -> A ? E ! Scheduler | N - Scheduler - Runner - request.Await` |
| map_concurrently | `(List(A), (A) -> B ? E ! N) -> List(B) ? E ! Scheduler | N - Scheduler - Runner - request.Await` |

Scheduler, Runner, Scope, Task, Channel, Event and Shared are public abstract
custom types. Only ScheduleError exports its NotRunnable constructor. The key
parameters of Scope/Task/Channel/Shared are invariant. Event's three parameters
are covariant; negative/positive spec tests check these independently of the
printed golden.

Printed differences and their meaning:

- The checker names the random module `rng`; this is the source import alias
  for `i/random`, so `rng.Generator` is `random.Generator` in the design table.
- Scope/join/map schemes rename bound variables by inference order. There is
  no change to quantification, variance or the relationship of their positions.
- go, keep, both, race and map_concurrently print the body environment as an
  intersection with `{ Scheduler | Runner | request.Await | N - Scheduler -
  Runner - request.Await }`, sometimes twice with a different member order.
  Let D be those three fixed routing services. `N & (D | (N - D)) = N`:
  every member of N is either in D or outside D. A second intersection is
  idempotent. This is the checker's unsimplified display of retaining the
  environment and then substituting routing, not an extra requirement.
- run/test have the same display identity with D extended by Root and Owner,
  matching §6.1's sentence “run/test supply a fresh Root and default Owner,
  so they subtract both.” The result environment still retains Runner.
- Public functions keep Assert and arbitrary user services exactly where the
  source table says N includes them. No host namespace has special treatment.

The entry classifier is `Lowerer::services_require_async(services: &[TypeId])`.
Per the coordinator's R2/JP ruling it receives the inferred environment now.
JP can pass the chosen provider closure to that same function. Provider tables
are rejected with E0522. The inherited JP clauses are listed in r2-stages.md;
they do not change any of the schemes above.
