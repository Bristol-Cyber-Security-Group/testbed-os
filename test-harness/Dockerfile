FROM python:3.13.3-slim-bookworm
LABEL authors="drrtdrrt"

WORKDIR /app
COPY requirements.txt .
# we need some packages to be able to install libvirt-python
RUN apt-get update && \
    apt-get install pkg-config libvirt-dev gcc -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
# install the test harness python environment
RUN pip install --no-cache-dir -r requirements.txt

# copy in all scripts and artefacts for running the test harness
COPY py_harness .
COPY test_cases test_cases
COPY assets assets

CMD ["python3", "main.py"]
