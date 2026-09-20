'use strict';
// Compatibility entry point for the initial prototype.
const lab = require('../../cli/src/observer-lab.cjs');
module.exports = lab;
if (require.main === module) console.log(JSON.stringify(lab.run(), null, 2));
