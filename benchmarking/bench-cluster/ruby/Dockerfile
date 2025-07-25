FROM ruby:3.3

WORKDIR /app

COPY . .

RUN gem install statsig

CMD ["ruby", "main.rb"]