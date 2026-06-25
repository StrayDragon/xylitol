# xylitol

> TODO: 等需求稳定在写

## Acknowledgments

This project draws inspiration from [pi](https://pi.dev), a powerful coding agent CLI/SDK. The following ideas and patterns were adapted from pi:

- **Version checking**: The update-check mechanism (`src/infra/update/`) was inspired by pi's approach to checking for newer versions via a remote API.
- **Provider attribution**: The concept of injecting attribution headers for providers like OpenRouter, NVIDIA, Cloudflare, and Vercel (see `llmanspec/changes/archive/2026-06-20-c82-provider-attribution/`) was adapted from pi's `provider-attribution.ts`.
- **Provider display names**: The mapping of provider IDs to human-readable names was inspired by pi's `provider-display-names.ts`.
- **Architecture patterns**: Various structural patterns in the agent loop, tool system, and configuration management draw from pi's well-designed SDK.

We gratefully acknowledge the pi team for their excellent work and for making their code available as a reference.

## License

MIT
