PY ?= python3
RESULTS_DIR ?= ../results

.PHONY: all stats figures clean

all: stats figures

stats:
	$(PY) stats.py $(RESULTS_DIR)

figures:
	$(PY) figures.py $(RESULTS_DIR)

clean:
	rm -rf figures
