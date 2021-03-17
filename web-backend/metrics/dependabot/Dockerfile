FROM ruby:2.6.6-buster

RUN gem install bundler

COPY Gemfile Gemfile.lock ./
RUN bundle install

COPY . .

CMD ["ruby", "changelog.rb"]
