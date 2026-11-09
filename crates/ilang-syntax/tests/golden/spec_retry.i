retry = (times: Int, body: () -> a ? e ! n) -> a ? e ! n {
  case fail.catch(body) {
    Ok(value) -> value
    Error(error) -> case times > 1 {
      True -> retry(times - 1, body)
      False -> fail.raise(error)
    }
  }
}

user = retry(3, () { load_user("1") })
