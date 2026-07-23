import { readFile, writeFile } from "node:fs/promises";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error("用法：npm run version:set -- 0.2.0");
  process.exit(1);
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

const packageJson = await readJson("package.json");
packageJson.version = version;
await writeJson("package.json", packageJson);

const packageLock = await readJson("package-lock.json");
packageLock.version = version;
if (packageLock.packages?.[""]) {
  packageLock.packages[""].version = version;
}
await writeJson("package-lock.json", packageLock);

const tauriConfig = await readJson("src-tauri/tauri.conf.json");
tauriConfig.version = version;
await writeJson("src-tauri/tauri.conf.json", tauriConfig);

const cargoPath = "src-tauri/Cargo.toml";
const cargoToml = await readFile(cargoPath, "utf8");
const cargoVersionPattern = /(\[package\][\s\S]*?\nversion = ")[^"]+(")/;
if (!cargoVersionPattern.test(cargoToml)) {
  throw new Error("未找到 src-tauri/Cargo.toml 中的 package version");
}
const updatedCargoToml = cargoToml.replace(
  cargoVersionPattern,
  `$1${version}$2`,
);
await writeFile(cargoPath, updatedCargoToml);

console.log(`ShowJSON 版本已同步为 ${version}`);
