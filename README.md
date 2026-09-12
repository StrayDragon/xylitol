# xylitol

## Acknowledgments

This project draws inspiration from [pi](https://pi.dev), a powerful coding agent CLI/SDK. The following ideas and patterns were adapted from pi:

- **Terminal UI (`packages/xylitol-tui`)**: Derived from [@earendil-works/pi-tui](https://github.com/earendil-works/pi/tree/main/packages/tui) (MIT, Copyright (c) 2025 Mario Zechner). Started as a Rust rewrite; maintained as an **independent fork** for xylitol and expected to diverge. See [`packages/xylitol-tui/NOTICE`](packages/xylitol-tui/NOTICE) and [`PI_DELTAS.md`](packages/xylitol-tui/PI_DELTAS.md).
- **Provider attribution**: The concept of injecting attribution headers for providers like OpenRouter, NVIDIA, Cloudflare, and Vercel (frozen change `c82-provider-attribution`; retrieve via `llman sdd archive freeze --list`) was adapted from pi's `provider-attribution.ts`.
- **Provider display names**: The mapping of provider IDs to human-readable names was inspired by pi's `provider-display-names.ts`.
- **Architecture patterns**: Various structural patterns in the agent loop, tool system, and configuration management draw from pi's well-designed SDK.

We gratefully acknowledge Mario Zechner and the pi / earendil-works contributors for making their work available under the MIT License.

## License

MIT — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
