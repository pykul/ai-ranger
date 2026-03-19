## What type of change is this?

- [ ] New provider
- [ ] Bug fix
- [ ] Feature
- [ ] Documentation
- [ ] Other

## What changed and why?

<!-- One or two sentences describing the change and its motivation. -->

## Provider additions

<!-- Skip this section if not adding a provider. -->

- **Provider name:** <!-- e.g., openai -->
- **Display name:** <!-- e.g., OpenAI -->
- **Hostnames added:** <!-- e.g., api.openai.com, *.openai.com -->
- **How hostname was found:** <!-- e.g., browser dev tools, network capture, docs -->
- **Tested locally:** yes / no

## Checklist

- [ ] CI passes
- [ ] For provider PRs: `cargo test` passes with the new provider recognized
- [ ] For code changes: tests added or updated
- [ ] No hardcoded values — constants used throughout
- [ ] Documentation updated if behavior changed
