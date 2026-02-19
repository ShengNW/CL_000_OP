# Step 6 Local Patches (Non-upstream)

1) `third_party/OmniParser/util/utils.py`
   - Make `openai` import optional (avoid hard crash when package not installed).
   - Make `paddleocr` optional and fall back to EasyOCR when missing.
   - Respect `EASYOCR_HOME` for model cache placement.
   - Lazy-init EasyOCR and return empty OCR results if init fails (keeps parse alive on torch 1.12).
   - Normalize `filtered_boxes` to dict form when OCR is empty (avoids list/dict mismatch).
   - Guard PaddleOCR init and fall back to EasyOCR if paddle args mismatch.
   - Force EasyOCR to CPU to avoid float/half mismatch on some CUDA stacks.
   - Coerce EasyOCR models to float32 and default dtype to float32.
   - Fallback to EasyOCR if PaddleOCR runtime fails.
2) `third_party/OmniParser/util/omniparser.py`
   - Catch caption model load failures (Florence2 / transformers mismatch) and fall back to `use_local_semantics=False`.
   - Prefer PaddleOCR when available to avoid EasyOCR GPU dtype mismatch.
   - Reason: allow OmniParser to run in Aliyun39 without PaddleOCR or OpenAI deps.
