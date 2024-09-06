import init, { codegen, Lang } from './pkg/jsoncodegen_web.js';

async function run() {
    await init();

    const generateBtn = document.getElementById('generate-btn');
    const jsonInput = document.getElementById('json-input');
    const output = document.getElementById('code-output');
    const langSelect = document.getElementById('lang-select');

    generateBtn.addEventListener('click', () => {
        const json = jsonInput.value;
        const lang = langSelect.value === "Java" ? Lang.Java : Lang.Rust;

        try {
            const code = codegen(json, lang);
            output.value = code;
        } catch (e) {
            output.value = `Error: ${e}`;
        }
    });
}

run();