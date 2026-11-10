//// The module documentation.
//// It has two lines.

 (i/core, decode = json/decode, opt = (Option, Some) = std/option,)
 (main, User(..),)

main = () { core.print(decode.run("1")) }
