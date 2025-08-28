FROM --platform=linux/amd64 debian:11-slim

RUN apt-get update && \
    apt-get install -y \
    php php-curl php-xml composer unzip php-cli php-json php-zip php-ffi

ARG RELEASE_TAG
ENV RELEASE_TAG=${RELEASE_TAG}

WORKDIR /app

RUN echo "ffi.enable=1" >> /etc/php/7.4/cli/conf.d/docker-php-ext-ffi.ini

COPY composer.json composer.lock ./
COPY install.sh .
RUN chmod +x install.sh && ./install.sh

COPY . .

RUN composer install --no-interaction --no-progress
RUN php --version

CMD ["php", "main.php"]
