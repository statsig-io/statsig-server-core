const fs = require('fs');
const path = require('path');

const GENERATED_FILE = 'src/lib/statsig-generated.d.ts';
const FILES_TO_APPEND = ['src/lib/additional_types.ts'];
const generatedFileFullPath = path.resolve(__dirname, '..', GENERATED_FILE);

FILES_TO_APPEND.forEach((filePath) => {
  const fullPath = path.resolve(__dirname, '..', filePath);
  let header = `// ---- Manually defined typing section ----- \n`;
  if (fs.existsSync(fullPath)) {
    let content = fs.readFileSync(fullPath, 'utf8');
    content = content.replace(
      /^\s*import\s+.*?from\s+['"]\.\/statsig-generated['"];\s*$/gm,
      '',
    );
    content = header + content;
    fs.appendFileSync(generatedFileFullPath, content, 'utf8');
  } else {
    console.warn(`❌ File not found: ${filePath}`);
  }
});
