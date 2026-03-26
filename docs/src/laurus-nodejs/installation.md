# Installation

## From npm

```bash
npm install laurus-nodejs
```

## From source

Building from source requires a Rust toolchain (1.85 or later)
and Node.js 18+.

```bash
# Clone the repository
git clone https://github.com/mosuka/laurus.git
cd laurus/laurus-nodejs

# Install dependencies
npm install

# Build the native module (release)
npm run build

# Or build in debug mode (faster builds)
npm run build:debug
```

## Verify

```javascript
import { Index } from "laurus-nodejs";
const index = await Index.create();
console.log(index.stats());
// { documentCount: 0, vectorFields: {} }
```

## Requirements

- Node.js 18 or later
- No runtime dependencies beyond the compiled native addon
