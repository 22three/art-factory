/*
namespace: Compile
expectation: Pass
input_file: inputs/true_true.in
*/

function main(a: (bool, bool)) -> (bool, bool) {
    let (b, c) = (a.0 ? false : true, a.1 ? false : true);

    return (c, b);
}
