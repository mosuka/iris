/**
 * Synonym Graph Filter example for laurus-nodejs.
 *
 * Demonstrates:
 * - Creating a synonym dictionary with bidirectional synonym groups
 * - Tokenizing text with WhitespaceTokenizer
 * - Applying SynonymGraphFilter without boost
 * - Applying SynonymGraphFilter with boost < 1.0 (synonyms weighted down)
 *
 * Run with:
 *     npm run build:debug
 *     node examples/synonym-graph-filter.mjs
 */

import {
  SynonymDictionary,
  WhitespaceTokenizer,
  SynonymGraphFilter,
} from "../index.js";

console.log("=== SynonymGraphFilter Usage Example ===\n");

// -- Step 1: Create synonym dictionary ---------------------------------------
console.log("Step 1: Creating synonym dictionary");
const synDict = new SynonymDictionary();
synDict.addSynonymGroup(["esm", "ECMAScript modules"]);
synDict.addSynonymGroup(["ts", "TypeScript"]);

console.log("  Added synonyms:");
console.log("    - 'esm' <-> 'ECMAScript modules'");
console.log("    - 'ts' <-> 'TypeScript'\n");

const tokenizer = new WhitespaceTokenizer();

// -- Step 2: Apply filter WITHOUT boost --------------------------------------
console.log("Step 2: Applying filter WITHOUT boost");
const filter = new SynonymGraphFilter(synDict, true, 1.0);

const inputText = "esm tutorial";
console.log(`  Input: "${inputText}"\n`);

const tokens = tokenizer.tokenize(inputText);
const resultTokens = filter.apply(tokens);

console.log("  Output tokens:");
for (let i = 0; i < resultTokens.length; i++) {
  const tok = resultTokens[i];
  console.log(
    `    [${i}] ${JSON.stringify(tok.text).padEnd(25)}  ` +
      `pos=${tok.position}  ` +
      `pos_inc=${tok.positionIncrement}  ` +
      `pos_len=${tok.positionLength}  ` +
      `boost=${tok.boost.toFixed(2)}`,
  );
}

console.log();
console.log("  Explanation:");
console.log("    - All tokens have boost=1.0 (default)");
console.log("    - Synonyms have equal weight to original tokens\n");

// -- Step 3: Apply filter WITH boost=0.8 -------------------------------------
console.log("Step 3: Applying filter WITH boost=0.8");
const filterBoosted = new SynonymGraphFilter(synDict, true, 0.8);

console.log(`  Input: "${inputText}"\n`);

const tokens2 = tokenizer.tokenize(inputText);
const resultTokens2 = filterBoosted.apply(tokens2);

console.log("  Output tokens:");
for (let i = 0; i < resultTokens2.length; i++) {
  const tok = resultTokens2[i];
  console.log(
    `    [${i}] ${JSON.stringify(tok.text).padEnd(25)}  ` +
      `pos=${tok.position}  ` +
      `pos_inc=${tok.positionIncrement}  ` +
      `pos_len=${tok.positionLength}  ` +
      `boost=${tok.boost.toFixed(2)}`,
  );
}

console.log();
console.log("  Explanation:");
console.log("    - Original token 'esm' has boost=1.0");
console.log("    - Synonym tokens have boost < 1.0");
console.log("    - Lower boost means synonyms contribute less to the final score");
console.log("    - This helps prioritize exact matches over synonym matches\n");

// -- Step 4: Multi-word synonym expansion ------------------------------------
console.log("Step 4: Multi-word synonym expansion (ts -> TypeScript)");
const filter2 = new SynonymGraphFilter(synDict, true, 0.9);

const inputText2 = "ts compiler";
console.log(`  Input: "${inputText2}"\n`);

const tokens3 = tokenizer.tokenize(inputText2);
const resultTokens3 = filter2.apply(tokens3);

console.log("  Output tokens (position graph):");
for (let i = 0; i < resultTokens3.length; i++) {
  const tok = resultTokens3[i];
  console.log(
    `    [${i}] ${JSON.stringify(tok.text).padEnd(25)}  ` +
      `pos=${tok.position}  ` +
      `pos_inc=${tok.positionIncrement}  ` +
      `pos_len=${tok.positionLength}  ` +
      `boost=${tok.boost.toFixed(2)}`,
  );
}

console.log();
console.log("  Explanation:");
console.log("    - 'TypeScript' is inserted as a synonym at the same position as 'ts'");
console.log("    - position_length indicates the span of positions the token covers");
console.log("    - Downstream phrase indexing uses this graph to support phrase queries\n");

console.log("Use cases for boost:");
console.log("  - boost=0.8: Synonyms have 80% weight (common default)");
console.log("  - boost=0.5: Synonyms have 50% weight (conservative)");
console.log("  - boost=1.0: Synonyms equal to originals (no adjustment)\n");

console.log("SynonymGraphFilter example completed!");
