import { expect, smoke } from "smoque";

const PRIVATE_CANARY = "postgres://private-ballast-token";

smoke.suite("packaged Ballast consumer", { tags: ["package", "http"] }, async (t) => {
  const root = t.repoRoot();
  const core = t.env.path("RECOURSE_CORE_PACKAGE", { required: true });
  const axum = t.env.path("RECOURSE_AXUM_PACKAGE", { required: true });
  const target = t.env.path("RECOURSE_PACKAGE_TARGET", { required: true });
  const work = await t.tempDir("recourse-package-consumer");
  const consumer = work.path("ballast-consumer");

  await t.step("prepare extracted-package consumer", async () => {
    await t.fs.copy(root.path("smoke/ballast-consumer"), consumer);
    const template = await t.fs.readText(`${consumer}/Cargo.toml.template`);
    const manifest = template
      .replaceAll("@RECOURSE_PATH@", core.toString())
      .replaceAll("@RECOURSE_AXUM_PATH@", axum.toString());
    await t.fs.writeText(`${consumer}/Cargo.toml`, manifest);
  });

  const port = await t.ports.reserve("ballast-consumer");
  const service = await t.step("start packaged Axum service", async () => {
    return await t.process.start(
      "cargo",
      ["run", "--manifest-path", `${consumer}/Cargo.toml`, "--offline", "--quiet"],
      {
        cwd: root,
        env: t.ports.env({ CARGO_TARGET_DIR: target.toString(), PORT: port }),
        name: "packaged-ballast-consumer",
        ready: t.http.ready(port.url("/ready")),
        timeout: "5m",
      },
    );
  });

  await t.step("public Problem crosses the real HTTP boundary", async () => {
    const response = await t.http.get(port.url("/deployments/dep_missing"), {
      headers: { "x-request-id": "ballast-smoke-request" },
    });

    response
      .expectStatus(404)
      .expectHeader("content-type").toBe("application/problem+json")
      .expectHeader("x-request-id").toBe("ballast-smoke-request")
      .expectJsonPath("$.type").toBe("https://ballast.invalid/problems/BAL-1001")
      .expectJsonPath("$.code").toBe("BAL-1001")
      .expectJsonPath("$.status").toBe(404)
      .expectJsonPath("$.evidence.deployment_id").toBe("dep_missing");
  });

  await t.step("private fault detail stays off the public wire", async () => {
    const response = await t.http.get(port.url("/fault"), {
      headers: { "x-request-id": "ballast-fault-request" },
    });

    response.expectStatus(500).expectJsonPath("$.code").toBe("BAL-1002");
    expect.value(response.body.includes(PRIVATE_CANARY)).toBe(false);
    expect.value(service.stderr()).toContain(`private-report: ${PRIVATE_CANARY}`);
  });

  await t.step("started stream emits a final encoded Problem frame", async () => {
    const response = await t.http.get(port.url("/stream"));
    response.expectStatus(200).expectHeader("content-type").matching(/^text\/event-stream/u);
    const data = response.body
      .split("\n")
      .find((line) => line.startsWith("data: "))
      ?.slice("data: ".length);
    if (data === undefined) {
      t.fail("SSE response omitted its Problem data frame");
    }
    const problem = JSON.parse(data) as Record<string, unknown>;
    expect.value(problem.code).toBe("BAL-1001");
    expect.value(problem.status).toBe(404);
    expect.value(service.stdout()).toContain("observed-problem: BAL-1001");
  });
});

smoke.suite("installed Recourse CLI", { tags: ["package", "cli"] }, async (t) => {
  const core = t.env.path("RECOURSE_CORE_PACKAGE", { required: true });
  const cli = t.env.path("RECOURSE_CLI_PACKAGE", { required: true });
  const target = t.env.path("RECOURSE_PACKAGE_TARGET", { required: true });
  const version = t.env.string("RECOURSE_RELEASE_VERSION", { required: true });
  const work = await t.tempDir("recourse-cli-install");
  const install = work.path("install");

  await t.step("install extracted CLI package", async () => {
    await t.cmd(
      "cargo",
      [
        "install",
        "--path", cli.toString(),
        "--root", install,
        "--offline",
        "--quiet",
        "--config", `patch.crates-io.recourse.path=\"${core.toString()}\"`,
      ],
      { env: { CARGO_TARGET_DIR: target.toString() }, timeout: "5m" },
    );
  });

  await t.step("installed binary reports release identity", async () => {
    const result = await t.cmd(`${install}/bin/cargo-recourse`, ["--version"]);
    expect.value(result.stdout.trim()).toBe(`cargo-recourse ${version}`);
  });
});
