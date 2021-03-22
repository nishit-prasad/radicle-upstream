import commonjs from "@rollup/plugin-commonjs";
import externals from "rollup-plugin-node-externals";
import typescript from "@rollup/plugin-typescript";

export default {
  input: "native/main.ts",
  output: {
    sourcemap: true,
    file: "native/main.comp.js",
    format: "cjs",
  },
  plugins: [
    commonjs(),

    typescript(),

    // This avoids the following warning:
    //
    // (!) Unresolved dependencies
    // https://rollupjs.org/guide/en/#warning-treating-module-as-external-dependency
    externals({ builtins: true, deps: true }),
  ],
};
