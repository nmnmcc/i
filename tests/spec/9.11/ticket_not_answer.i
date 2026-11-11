// spec: 9.11
// expect: compile-error E0501
// runtime.md D2: a Ticket cannot read an answer.
(i/request)

main = () { request.take(request.ticket(request.ready(37))) }
