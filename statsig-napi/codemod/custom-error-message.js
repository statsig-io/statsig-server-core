module.exports = function (fileInfo, api) {
  const j = api.jscodeshift;
  const root = j(fileInfo.source);

  let didApplyCodemod = false;

  root
    .find(j.IfStatement, {
      test: {
        type: 'UnaryExpression',
        operator: '!',
        argument: {
          type: 'Identifier',
          name: 'nativeBinding',
        },
      },
    })
    .filter((path) => {
      const consequent = path.value.consequent;
      if (
        consequent.type === 'BlockStatement' &&
        consequent.body.length === 2 &&
        consequent.body[0].type === 'IfStatement' &&
        consequent.body[0].test.type === 'BinaryExpression' &&
        consequent.body[0].test.operator === '>' &&
        consequent.body[0].test.left.object.name === 'loadErrors'
      ) {
        return true;
      }
      return false;
    })
    .replaceWith(() => {
      didApplyCodemod = true;
      return j.template.statement`
if (!nativeBinding) {
  const plat = process.platform;
  const arch = process.arch;
  const message = '[Statsig]: Native library not found for ' 
    + plat + '-' + arch + (isMusl() ? '-musl' : '');

  if (loadErrors.length > 0) {
    // TODO Link to documentation with potential fixes
    //  - The package owner could build/publish bindings for this arch
    //  - The user may need to bundle the correct files
    //  - The user may need to re-install node_modules to get new packages
    
    throw new Error(message, { cause: loadErrors });
  }
  throw new Error(message);
}
`;
    });

  if (!didApplyCodemod) {
    throw new Error(
      'Statsig Build Error: Could not find napi bindings error message',
    );
  }

  return root.toSource();
};
