REPO ?= spalencsar/deskify

.PHONY: labels
labels:
	./scripts/setup-github-labels.sh $(REPO)
