# SUNDAY MTP Lab

ทดลอง Gemma 4 draft/MTP แบบแยกจากระบบหลัก

## Layout

- `atomic-llama-cpp-turboquant/` - source fork, สร้างโดย `setup_atomic_mtp.ps1`
- `build/` - build output ของ Atomic fork
- `setup_atomic_mtp.ps1` - clone/update และ build binary
- `start_mtp_voice_llm.ps1` - เปิด MTP server แยกที่ port 8084
- `start_draft_voice_llm.ps1` - เปิด draft/speculative server ด้วย `llama-cpp` เดิมที่ port 8084
- `test_mtp_voice_llm.ps1` - ยิง chat completion สั้น ๆ เพื่อวัดว่า server ใช้ได้

## Models

ใช้ official/instruct target ก่อน ถ้ามี:

```text
llama-cpp/models/gemma-4-E4B-it-Q4_K_M.gguf
```

ถ้าไม่มี สคริปต์จะ fallback ไปที่ `gemma-4-E4B-it-ultra-uncensored-heretic-Q4_K_M.gguf`.

Atomic/TurboQuant `--mtp-head` ต้องใช้ GGUF ที่ metadata เป็น `gemma4_assistant`.

ไฟล์ HackAfterDark ultralight นี้เป็น arch `gemma4`:

```text
llama-cpp/models/gemma-4-e4b-it-mtp-assistant-ultralight.f16.gguf
```

จึงใช้กับ `--mtp-head` ไม่ได้โดยตรง แต่ใช้เป็น draft model ผ่าน `start_draft_voice_llm.ps1` ได้

Atomic path ต้องมีไฟล์ assistant ที่เข้ากันได้ เช่น:

```text
llama-cpp/models/gemma-4-E4B-it-assistant.Q4_K_M.gguf
```

เปิดด้วย Atomic fork เมื่อมีไฟล์ `gemma4_assistant` แล้ว:

```powershell
.\mtp-lab\setup_atomic_mtp.ps1
.\mtp-lab\start_mtp_voice_llm.ps1
.\mtp-lab\test_mtp_voice_llm.ps1
```

สคริปต์จะใช้:

- `--spec-type mtp`
- `--mtp-head`
- `--draft-block-size 3`
- `-ctk/-ctv/-ctkd/-ctvd turbo3`
- `--swa-full`

ถ้าจะลองไฟล์ assistant Q4 แบบอื่น ให้มี MTP assistant head ของ Gemma 4 E4B เพิ่ม:

```text
llama-cpp/models/gemma-4-E4B-it-assistant-Q4_K_M.gguf
```

แนะนำให้โหลดจาก collection `AtomicChat/gemma-4-assistant-GGUF` หรือ repo MTP assistant ที่ตรงกับ Gemma 4 E4B.

ยังมีทาง fallback ที่ใช้ `llama-cpp` เดิม:

```powershell
.\mtp-lab\start_draft_voice_llm.ps1
```

ทางนี้ใช้ HackAfterDark ultralight ตามแนว `--model-draft`:

- target: `gemma-4-E4B-it-Q4_K_M.gguf`
- draft: `gemma-4-e4b-it-mtp-assistant-ultralight.f16.gguf`
- `--swa-full`
- `--kv-unified`
- `--cache-type-k q4_0`
- `--cache-type-v q4_0`
- `--spec-draft-n-max 8`
- `-b 128 -ub 128`

ค่า context default ของ lab คือ `4096` เพราะ RTX 4050 6GB เล็กกว่า RTX 3060 12GB ในตัวอย่าง upstream. ถ้าจะลองยาวขึ้น:

```powershell
$env:SUNDAY_MTP_CONTEXT_SIZE="8192"
.\mtp-lab\start_draft_voice_llm.ps1
```

## Run

```powershell
.\mtp-lab\setup_atomic_mtp.ps1
.\mtp-lab\start_mtp_voice_llm.ps1
.\mtp-lab\test_mtp_voice_llm.ps1
```

ถ้าทดสอบผ่าน ให้หน้า Voice Live ชี้ `Voice LLM` ไปที่:

```text
http://127.0.0.1:8084/v1/chat/completions
```

ระบบหลักยังใช้ port เดิมได้ตามปกติ:

- Main SUNDAY model: `8081`
- Voice fast model: `8082`
- Voice overlay: `8098`
- MTP lab: `8084`
