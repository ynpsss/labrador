use crate::current_timestamp;
use super::ReplyRenderer;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Article {
    pub title: String,
    pub description: String,
    pub url: String,
    pub image: String,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ArticlesReply {
    pub source: String,
    pub target: String,
    pub time: i64,
    pub articles: Vec<Article>,
}

#[allow(dead_code)]
impl Article {

    #[inline]
    pub fn new<S: Into<String>>(title: S, url: S) -> Article {
        Article {
            title: title.into(),
            url: url.into(),
            image: "".to_owned(),
            description: "".to_owned(),
        }
    }

    #[inline]
    pub fn with_image<S: Into<String>>(title: S, url: S, image: S) -> Article {
        Article {
            title: title.into(),
            url: url.into(),
            image: image.into(),
            description: "".to_owned(),
        }
    }

    #[inline]
    pub fn with_description<S: Into<String>>(title: S, url: S, description: S) -> Article {
        Article {
            title: title.into(),
            url: url.into(),
            image: "".to_owned(),
            description: description.into(),
        }
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) -> &mut Self {
        self.title = title.into();
        self
    }

    pub fn set_url<S: Into<String>>(&mut self, url: S) -> &mut Self {
        self.url = url.into();
        self
    }

    pub fn set_image<S: Into<String>>(&mut self, image: S) -> &mut Self {
        self.image = image.into();
        self
    }

    pub fn set_description<S: Into<String>>(&mut self, description: S) -> &mut Self {
        self.description = description.into();
        self
    }

    fn render(&self) -> String {
        format!("<item>\n
            <Title><![CDATA[{title}]]></Title>\n\
            <Description><![CDATA[{description}]]></Description>\n\
            <PicUrl><![CDATA[{picurl}]]></PicUrl>\n\
            <Url><![CDATA[{url}]]></Url>\n\
            </item>",
            title=self.title,
            description=self.description,
            picurl=self.image,
            url=self.url,
        )
    }
}

#[allow(unused)]
impl ArticlesReply {
    #[inline]
    pub fn new<S: Into<String>>(source: S, target: S) -> ArticlesReply {
        ArticlesReply {
            source: source.into(),
            target: target.into(),
            time: current_timestamp(),
            articles: vec![],
        }
    }

    #[inline]
    pub fn with_articles<S: Into<String>>(source: S, target: S, articles: &[Article]) -> ArticlesReply {
        ArticlesReply {
            source: source.into(),
            target: target.into(),
            time: current_timestamp(),
            articles: articles.to_vec(),
        }
    }

    pub fn add_article(&mut self, article: Article) -> bool {
        if self.articles.len() >= 10 {
            return false;
        }
        self.articles.push(article);
        true
    }
}

impl ReplyRenderer for ArticlesReply {
    #[inline]
    fn render(&self) -> String {
        let mut articles = vec![];
        for article in self.articles.iter() {
            articles.push(article.render());
        }
        let articles_str = articles.join("\n");
        format!("<xml>\n\
            <ToUserName><![CDATA[{target}]]></ToUserName>\n\
            <FromUserName><![CDATA[{source}]]></FromUserName>\n\
            <CreateTime>{time}</CreateTime>\n\
            <MsgType><![CDATA[news]]></MsgType>\n\
            <ArticleCount>{count}</ArticleCount>\n\
            <Articles>{articles}</Articles>\n\
            </xml>",
            target=self.target,
            source=self.source,
            time=self.time,
            count=self.articles.len(),
            articles=articles_str,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ReplyRenderer;
    use super::{Article, ArticlesReply};

    #[test]
    fn test_render_articles_reply() {
        let mut reply = ArticlesReply::new("test1", "test2");
        let article1 = Article::new("test3", "test4");
        let article2 = Article::with_image("test5", "test6", "test7");
        let article3 = Article::with_description("test8", "test9", "test10");
        reply.add_article(article1);
        reply.add_article(article2);
        reply.add_article(article3);
        let rendered = reply.render();

        assert!(rendered.contains("test1"));
        assert!(rendered.contains("test2"));
        assert!(rendered.contains("test3"));
        assert!(rendered.contains("test4"));
        assert!(rendered.contains("test5"));
        assert!(rendered.contains("test6"));
        assert!(rendered.contains("test7"));
    }
}
