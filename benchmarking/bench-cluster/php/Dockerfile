FROM debian:11-slim

RUN apt-get update && \
    apt-get install -y \
    php php-curl php-xml composer unzip php-cli php-json php-zip php-ffi

RUN echo "ffi.enable=1" >> /etc/php/8.3/cli/conf.d/docker-php-ext-ffi.ini

WORKDIR /app

COPY . .
RUN composer install
RUN php --version

CMD ["php", "main.php"]
