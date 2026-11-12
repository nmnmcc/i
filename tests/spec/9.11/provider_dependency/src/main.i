Config = (value = Int)
Service = (read = () -> Int)
config = () { Config(value = 41) }
service = () <- (config: Config) { Service(read = () { config.value + 1 }) }
main = () <- (service: Service) { assert(service.read() == 42) }
