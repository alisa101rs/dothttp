pm.collectionVariables.set(
  "$random.float",
  Object.create({ toJSON: () => Math.random() * 1000.0 }),
);
pm.collectionVariables.set(
  "$random.float()",
  Object.create({ toJSON: () => Math.random() * 1000.0 }),
);

function Resolvable(value, name) {
  return {
    name,
    tryGetSubstituted: () => pm.variables.replaceIn(value),
    tryGetSubstitutedValue: () => pm.variables.replaceIn(value),
    getRaw: () => value,
    getRawValue: () => value,
  };
}

shared = {
  randomString: function (len, an) {
    an = an && an.toLowerCase();
    let str = "",
      i = 0,
      min = an == "a" ? 10 : 0,
      max = an == "h" ? 16 : 62;

    for (; i++ < len; ) {
      let r = (Math.random() * (max - min) + min) << 0;
      str += String.fromCharCode((r += r > 9 ? (r < 36 ? 55 : 61) : 48));
    }
    return str;
  },
  load(pm) {
    return {
      client: {
        log: function (...args) {
          console.log(...args);
        },
        global: {
          get: (name) => pm.globals.get(name),
          set: function (name, value) {
            pm.globals.set(name, value);
          },
          clear: function (name) {
            pm.globals.unset(name);
          },
          isEmpty: () => pm.globals.toObject().length === 0,
          clearAll: function () {
            pm.globals.clear();
          },
        },
      },
      request: {
        environment: {
          get: (name) => pm.variables.get(name),
        },
        url: new Resolvable(pm.request.url.toString()),
        headers: new (function () {
          const headers = pm.request.getHeaders();
          return { findByName: (name) => new Resolvable(headers[name], name) };
        })(),
        body: new Resolvable(
          pm.request.body.isEmpty() ? "null" : pm.request.body.toString(),
        ),
        variables: {
          get: (name) => pm.variables.get(name),
          set: function (name, value) {
            pm.variables.set(name, value);
          },
          clear: function (name) {
            pm.variables.unset(name);
          },
          isEmpty: () => pm.variables.toObject().length === 0,
          clearAll: function () {
            pm.variables.clear();
          },
        },
      },
    };
  },
};
