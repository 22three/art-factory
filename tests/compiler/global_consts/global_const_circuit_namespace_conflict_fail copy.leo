/*
namespace: Compile
expectation: Fail
input_file:
 - inputs/dummy.in
*/

const Foo = 8u8;

circuit Foo {
    x: u8;
}

function main(y: bool) -> bool {
    return y;
}