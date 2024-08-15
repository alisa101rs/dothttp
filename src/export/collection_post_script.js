shared = {
  load(pm) {
    const response = {
      status: pm.response.code,
      headers: Object.fromEntries(
        pm.response.headers.map(({ key, value }) => [key, value]),
      ),
      body: pm.response.text(),
    };
    try {
      response.body = pm.response.json();
    } catch (_) {}

    const client = {
      log: function (...args) {
        console.log(...args);
      },
      test: function (name, scope) {
        pm.test(name, scope);
      },
      assert: function (expr, message) {
        pm.expect(expr, message).to.eql(true);
      },
      global: {
        get: (name) => pm.globals.get(name),
        set: (...args) => pm.globals.set(...args),
        clear: function (name) {
          pm.globals.unset(name);
        },
        isEmpty: () => pm.globals.toObject().length === 0,
        clearAll: function () {
          pm.globals.clear();
        },
      },
    };
    return { client, response };
  },
};
