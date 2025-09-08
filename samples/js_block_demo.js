// Single-line comment in JS
/*
  Multi-line comment begins
  and continues here
*/

function main() {
  const x = 42; // inline comment
  const msg = "string with /* not a comment */ and // not a comment";
  /* another block comment */
  if (x > 0) {
    console.log(msg);
  }
}

main();

