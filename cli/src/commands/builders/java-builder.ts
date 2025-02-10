import { BuilderOptions } from "@/commands/builders/builder-options.js";
import { Log } from "@/utils/teminal_utils.js";
import { BASE_DIR } from "@/utils/file_utils.js";
import path from "node:path";
import fs from "fs";
import {buildFfiHelper} from "@/utils/ffi_utils.js";

const JAVA_NATIVE_DIR = path.resolve(
    BASE_DIR,
    'statsig-ffi/bindings/java/src/main/resources/native',
);

export function buildJava(options: BuilderOptions) {
    Log.title(`Building statsig-java`);

    options.release = true; // default to true
    buildFfiHelper(options);

    Log.stepEnd(`Built statsig-java`);

    moveNativeLibrary(options);
}


function moveNativeLibrary(options: BuilderOptions) {
    const targetDir = path.resolve(BASE_DIR, 'target/release');

    Log.stepBegin(`Moving FFI build output to Java native resources`);

    const files = [
        'libstatsig_ffi.so',
        'libstatsig_ffi.dylib',
        'libstatsig_ffi.dll'
    ];

    let found = false;

    // ensure empty dir or delete existing files
    if (fs.existsSync(JAVA_NATIVE_DIR)) {
        fs.readdirSync(JAVA_NATIVE_DIR).forEach(file => {
            fs.unlinkSync(path.join(JAVA_NATIVE_DIR, file));
        });
    } else {
        fs.mkdirSync(JAVA_NATIVE_DIR, { recursive: true });
    }

    for (const file of files) {
        const srcPath = path.join(targetDir, file);
        const destPath = path.join(JAVA_NATIVE_DIR, file);

        if (fs.existsSync(srcPath)) {
            fs.copyFileSync(srcPath, destPath);
            Log.stepEnd(`✅ Moved ${file} to ${JAVA_NATIVE_DIR}`);
            found = true;
            break; // Stop after moving the first valid file??
        }
    }

    if (!found) {
        Log.stepEnd(`❌ ERROR: No native library found! Build failed.`);
        process.exit(1);
    }
}
