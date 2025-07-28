FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app

RUN pip install --no-cache-dir statsig-python-core --pre
RUN pip install --no-cache-dir requests statsig

COPY . .

CMD ["python", "main.py"]
