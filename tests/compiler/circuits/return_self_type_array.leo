/*
namespace: Compile
expectation: Pass
input_file: input/dummy.in
*/

circuit Circ {
    function fn () -> [Self; 1] {
        return [Self {}];
    }
}

function main (y: bool) -> bool {
	return y;
}