(
	Config(..)
	ConfigType
	config
	Method
	Port
	Number
	Methods
)

Config(M, P) = (method = M, port = P)
config = Config(method = "GET", port = 8080)
ConfigType = config

method = "GET"
Method = method
port = 8080
Port = port

Box(A) = (A)
boxed = True |> (True) { Box(1) } | (False) { Box(2) }
Box(Number) = boxed

methods: "GET" | "POST" = "GET"
Methods = methods
