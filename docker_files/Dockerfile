FROM node:lts-slim

USER root

VOLUME /rusty-rep

ADD test.sh /

RUN touch main.js && npm init -y

CMD /test.sh && tail -f /dev/null
