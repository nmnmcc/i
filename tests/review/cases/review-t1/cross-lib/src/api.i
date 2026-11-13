(
	Modes
	last
	identity
)
Modes = "v00"
	| "v01"
	| "v02"
	| "v03"
	| "v04"
	| "v05"
	| "v06"
	| "v07"
	| "v08"
	| "v09"
	| "v10"
	| "v11"
	| "v12"
	| "v13"
	| "v14"
	| "v15"
	| "v16"
	| "v17"
	| "v18"
	| "v99"
last = "v99"
identity = (m: Modes) { m }
