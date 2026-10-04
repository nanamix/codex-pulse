# 의존성 상태

확인일: 2026-10-04. GitHub Dependabot가 Linux GUI의 간접 의존성인 `glib 0.18.5`에 대해 [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g)를 보고했습니다. 공식 수정 버전은 0.20.0이며, 현재 Tauri/GTK 의존성은 0.18 계열을 사용합니다.

```sh
cargo tree --workspace -i glib --target x86_64-unknown-linux-gnu --locked
```

위 명령으로 Tauri 2.12.1, GTK 0.18.2 및 WebKitGTK 경유 의존성을 확인했습니다. 앱 소스는 `VariantStrIter`를 직접 사용하지 않습니다. 간접 실행 경로의 영향은 검증하지 않았으므로 안전하거나 해결됐다고 주장하지 않습니다.

이 릴리스는 해당 경고를 해결하지 않았습니다. 서로 다른 GTK 바인딩 버전으로 단순 강제 교체하지 않았으며, 경고를 숨기거나 dismiss하지 않았습니다. 후속 조치에는 Tauri/GTK 호환 업그레이드 또는 upstream 수정의 검증된 backport가 필요합니다.
