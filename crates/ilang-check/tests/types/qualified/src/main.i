// §3: a type of another module is shown with the name its module has here,
// or with the module path when it is not imported. A type imported by
// name is shown by that name.
(
	m = app/other
	(Shown) = app/other
)

(
	both
	shown
	far
	hidden
	T
)

T = (String)

H = ()

both = (m.mk(1), T("s"))

shown = Shown

far = m.far()

// §2.4: the inferred type of an exported binding may not use a type of
// this module that is not exported.
hidden = () { H }

local = () { H }
