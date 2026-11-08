// Read-only compatibility probe against an explicitly supplied original checkout.
// Usage: bun scripts/probe-legacy-editor-statistics.ts LEGACY_CHECKOUT OUTPUT_JSON
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const legacyRoot = process.argv[2];
const outputPath = process.argv[3];
if (!legacyRoot || !outputPath) throw new Error('Supply the original checkout and output JSON path.');
const { getSchema, getText, getTextSerializersFromSchema } = await import(pathToFileURL(resolve(legacyRoot, 'node_modules/@tiptap/core/dist/index.js')).href);
const { editorExtensions } = await import(pathToFileURL(resolve(legacyRoot, 'src/features/editor/model/extensions.ts')).href);
const schema = getSchema(editorExtensions);
const t = (text: string) => ({type:'text',text,marks:[{type:'bold'},{type:'italic'},{type:'underline'},{type:'strike'},{type:'link',attrs:{href:'https://example.org'}}]});
const p = (id:string,text:string) => ({type:'paragraph',attrs:{blockId:id},content:[t(text)]});
const fixtures = [
  {name:'paragraphs-hardbreak-inline-math', content:[{type:'paragraph',attrs:{blockId:'p'},content:[t('Я 😀 é'),{type:'text',text:' code',marks:[{type:'code'}]},{type:'hardBreak'},{type:'inlineMath',attrs:{latex:'hidden_formula',description:'hidden math description'}},t('After')]},p('p2','two\ufeffthree\u0085four')]},
  {name:'nested-container-and-table',content:[{type:'blockquote',attrs:{blockId:'quote'},content:[p('qp','Quote')]},{type:'bulletList',content:[{type:'listItem',content:[p('li','List')]}]},{type:'orderedList',attrs:{start:2},content:[{type:'listItem',content:[p('ol','Ordered')]}]},{type:'callout',attrs:{kind:'idea',blockId:'callout'},content:[p('cp','Callout')]},{type:'table',attrs:{blockId:'table'},content:[{type:'tableRow',content:[{type:'tableHeader',content:[p('th','Head')]},{type:'tableCell',content:[p('td','Cell')]}]}]}]},
  {name:'headings-code-opaque-blocks',content:[{type:'heading',attrs:{level:6,blockId:'heading'},content:[t('Heading')]},{type:'horizontalRule'},{type:'sceneBreak'},{type:'imageBlock',attrs:{src:'assets/synthetic.png',blockId:'image',alt:'not counted alt',caption:'not counted image caption'}},{type:'mermaidBlock',attrs:{blockId:'mermaid',code:'flowchart LR; A-->B',caption:'not counted diagram caption'}},{type:'mathBlock',attrs:{blockId:'math',latex:'not_counted_formula',description:'not counted description'}},{type:'codeBlock',attrs:{language:'rust',blockId:'code'},content:[{type:'text',text:'fn main() {\n  x();\n}\n'}]}]},
];
const serializers = getTextSerializersFromSchema(schema);
const results = fixtures.map(({name,content}) => {
  const input={type:'doc',attrs:{schemaVersion:4},content};
  const doc = schema.nodeFromJSON(input);
  doc.check();
  const text=getText(doc,{textSerializers:serializers});
  const words=text.trim()?text.trim().split(/\s+/u).length:0;
  return {name,input,text,words,characters:Array.from(text).length,readingMinutes:words===0?0:Math.max(1,Math.ceil(words/180))};
});
writeFileSync(outputPath,JSON.stringify(results,null,2)+'\n');
console.log(JSON.stringify({registeredNodes:Object.keys(schema.nodes),registeredMarks:Object.keys(schema.marks),serializers:Object.keys(serializers),fixtures:results.map(({name,words,characters,readingMinutes})=>({name,words,characters,readingMinutes}))},null,2));
