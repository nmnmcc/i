Config { Config(value: Int) }
Service { Service(read: () -> Int) }
config = () { Config(41) }
service = (config ! Config) { Service(() { config.value + 1 }) }
main = (service ! Service) { assert(service.read() == 42) }
