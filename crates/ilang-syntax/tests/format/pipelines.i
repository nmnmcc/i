short = (xs) { xs |> list.map(f) |> list.sum }

long = (xs) {
  xs |> list.map(with: (x) { x * 2 }) |> list.filter(keeping: int.is_even) |> list.fold(from: 0, with: add)
}

compare = () {
  assert({ [1, 2, 3] |> set.from_list |> set.to_list |> list.sort(by: int.compare) |> list.reverse } == [3, 2, 1])
}
