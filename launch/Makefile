.PHONY: setup

VENV_NAME?=venv

setup:
	@python3 -m venv $(VENV_NAME)
	@source $(VENV_NAME)/bin/activate && \
		$(VENV_NAME)/bin/pip install --upgrade pip && \
		$(VENV_NAME)/bin/pip install -r requirements.txt
