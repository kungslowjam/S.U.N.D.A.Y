# SUNDAY Omni Lab

Isolated MiniCPM-o omni experiment area. Everything generated here is disposable.

## Layout

```text
omni-lab/
├── llama.cpp-omni/              # cloned runtime
├── models/MiniCPM-o-4_5-gguf/   # omni model modules
├── output/                      # generated wav/debug output
├── setup_omni_lab.ps1
├── build_omni.ps1
├── run_omni_text_only.ps1
├── run_omni_full.ps1
└── cleanup_omni_lab.ps1
```

## Steps

1. Prepare repo and model files:

```powershell
.\omni-lab\setup_omni_lab.ps1
```

2. Build runtime:

```powershell
.\omni-lab\build_omni.ps1
```

If CMake cannot find a compiler, run the command from **Developer PowerShell for Visual Studio**.

3. Try the lighter text-only omni path first:

```powershell
.\omni-lab\run_omni_text_only.ps1
```

4. Try full TTS/omni mode:

```powershell
.\omni-lab\run_omni_full.ps1
```

## Cleanup

```powershell
.\omni-lab\cleanup_omni_lab.ps1
```

This removes only generated `omni-lab/llama.cpp-omni`, `omni-lab/models`, `omni-lab/output`, and `omni-lab/build-logs`.

## Notes

Your RTX 4050 Laptop GPU has 6GB VRAM. Full Omni Q4_K_M is documented around 9GB VRAM, so full mode may spill to CPU or fail. Text-only or no-TTS mode is the first sanity test.
