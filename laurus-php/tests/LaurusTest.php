<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Basic integration tests for the laurus PHP binding.
 */
class LaurusTest extends TestCase
{
    // ── Helpers ──────────────────────────────────────────────────────────

    /**
     * Return a fresh in-memory index with two indexed documents.
     */
    private function createIndex(): Laurus\Index
    {
        $schema = new Laurus\Schema();
        $schema->addTextField("title");
        $schema->addTextField("body");
        $idx = new Laurus\Index(null, $schema);
        $idx->putDocument("doc1", ["title" => "Introduction to Rust", "body" => "Systems programming language."]);
        $idx->putDocument("doc2", ["title" => "Python for Data Science", "body" => "Data analysis with Python."]);
        $idx->commit();
        return $idx;
    }

    /**
     * Return an in-memory HNSW index with two indexed documents.
     */
    private function createVectorIndex(): Laurus\Index
    {
        $schema = new Laurus\Schema();
        $schema->addTextField("title");
        $schema->addHnswField("embedding", 4);
        $idx = new Laurus\Index(null, $schema);
        $idx->putDocument("doc1", ["title" => "Rust", "embedding" => [0.1, 0.2, 0.3, 0.4]]);
        $idx->putDocument("doc2", ["title" => "Python", "embedding" => [0.9, 0.8, 0.7, 0.6]]);
        $idx->commit();
        return $idx;
    }

    // ── Index creation ──────────────────────────────────────────────────

    public function testIndexMemory(): void
    {
        $idx = new Laurus\Index();
        $this->assertNotNull($idx);
    }

    public function testIndexWithSchema(): void
    {
        $schema = new Laurus\Schema();
        $schema->addTextField("title");
        $idx = new Laurus\Index(null, $schema);
        $this->assertNotNull($idx);
    }

    // ── Document CRUD ───────────────────────────────────────────────────

    public function testPutAndGetDocument(): void
    {
        $schema = new Laurus\Schema();
        $schema->addTextField("title");
        $idx = new Laurus\Index(null, $schema);
        $idx->putDocument("doc1", ["title" => "Hello"]);
        $idx->commit();
        $docs = $idx->getDocuments("doc1");
        $this->assertCount(1, $docs);
    }

    public function testPutReplacesExisting(): void
    {
        $idx = $this->createIndex();
        $idx->putDocument("doc1", ["title" => "Updated"]);
        $idx->commit();
        $docs = $idx->getDocuments("doc1");
        $this->assertCount(1, $docs);
    }

    public function testAddDocumentAppends(): void
    {
        $schema = new Laurus\Schema();
        $schema->addTextField("title");
        $idx = new Laurus\Index(null, $schema);
        $idx->addDocument("doc1", ["title" => "Chunk 1"]);
        $idx->addDocument("doc1", ["title" => "Chunk 2"]);
        $docs = $idx->getDocuments("doc1");
        $this->assertCount(2, $docs);
    }

    public function testDeleteDocuments(): void
    {
        $idx = $this->createIndex();
        $idx->deleteDocuments("doc1");
        $idx->commit();
        $docs = $idx->getDocuments("doc1");
        $this->assertCount(0, $docs);
    }

    public function testGetUnknownId(): void
    {
        $idx = $this->createIndex();
        $docs = $idx->getDocuments("unknown");
        $this->assertCount(0, $docs);
    }

    // ── Statistics ───────────────────────────────────────────────────────

    public function testDocumentCount(): void
    {
        $idx = $this->createIndex();
        $stats = $idx->stats();
        $this->assertEquals(2, $stats["document_count"]);
    }

    public function testVectorFieldStats(): void
    {
        $idx = $this->createVectorIndex();
        $stats = $idx->stats();
        $this->assertEquals(2, $stats["document_count"]);
        $this->assertArrayHasKey("embedding", $stats["vector_fields"]);
        $this->assertEquals(4, $stats["vector_fields"]["embedding"]["dimension"]);
    }

    // ── Lexical search ──────────────────────────────────────────────────

    public function testSearchDsl(): void
    {
        $idx = $this->createIndex();
        $results = $idx->search("title:rust");
        $this->assertCount(1, $results);
        $this->assertEquals("doc1", $results[0]->getId());
        $this->assertGreaterThan(0, $results[0]->getScore());
    }

    public function testSearchTermQuery(): void
    {
        $idx = $this->createIndex();
        $q = new Laurus\TermQuery("title", "python");
        $results = $idx->search($q);
        $this->assertCount(1, $results);
        $this->assertEquals("doc2", $results[0]->getId());
    }

    public function testSearchWithLimit(): void
    {
        $idx = $this->createIndex();
        $results = $idx->search("title:rust OR title:python", 1);
        $this->assertCount(1, $results);
    }

    public function testSearchWithOffset(): void
    {
        $idx = $this->createIndex();
        $all = $idx->search("title:rust OR title:python", 10);
        $offset = $idx->search("title:rust OR title:python", 10, 1);
        $this->assertCount(count($all) - 1, $offset);
    }

    public function testSearchNoResults(): void
    {
        $idx = $this->createIndex();
        $results = $idx->search("title:nonexistent");
        $this->assertCount(0, $results);
    }

    // ── Query types ─────────────────────────────────────────────────────

    public function testPhraseQuery(): void
    {
        $idx = $this->createIndex();
        $q = new Laurus\PhraseQuery("body", ["systems", "programming"]);
        $results = $idx->search($q);
        $this->assertCount(1, $results);
    }

    public function testFuzzyQuery(): void
    {
        $idx = $this->createIndex();
        $q = new Laurus\FuzzyQuery("title", "rast");
        $results = $idx->search($q);
        $this->assertCount(1, $results);
    }

    public function testBooleanQuery(): void
    {
        $idx = $this->createIndex();
        $bq = new Laurus\BooleanQuery();
        $bq->must(new Laurus\TermQuery("title", "rust"));
        $bq->mustNot(new Laurus\TermQuery("title", "python"));
        $results = $idx->search($bq);
        $this->assertCount(1, $results);
        $this->assertEquals("doc1", $results[0]->getId());
    }

    public function testWildcardQuery(): void
    {
        $idx = $this->createIndex();
        $q = new Laurus\WildcardQuery("title", "ru*");
        $results = $idx->search($q);
        $this->assertCount(1, $results);
    }

    public function testNumericRangeQuery(): void
    {
        $schema = new Laurus\Schema();
        $schema->addTextField("title");
        $schema->addIntegerField("year");
        $idx = new Laurus\Index(null, $schema);
        $idx->putDocument("d1", ["title" => "old", "year" => 2000]);
        $idx->putDocument("d2", ["title" => "new", "year" => 2024]);
        $idx->commit();
        $q = new Laurus\NumericRangeQuery("year", 2020, 2030);
        $results = $idx->search($q);
        $this->assertCount(1, $results);
        $this->assertEquals("d2", $results[0]->getId());
    }

    // ── Vector search ───────────────────────────────────────────────────

    public function testVectorQuery(): void
    {
        $idx = $this->createVectorIndex();
        $q = new Laurus\VectorQuery("embedding", [0.1, 0.2, 0.3, 0.4]);
        $results = $idx->search($q);
        $this->assertGreaterThanOrEqual(1, count($results));
        $this->assertEquals("doc1", $results[0]->getId());
    }

    // ── Hybrid search ───────────────────────────────────────────────────

    public function testSearchRequestLexicalOnly(): void
    {
        $idx = $this->createIndex();
        $req = new Laurus\SearchRequest(
            null, // query
            new Laurus\TermQuery("title", "rust"), // lexical_query
            null, // vector_query
            null, // filter_query
            null, // fusion
        );
        $results = $idx->search($req);
        $this->assertCount(1, $results);
    }

    // ── Fusion algorithms ───────────────────────────────────────────────

    public function testRRFRepr(): void
    {
        $rrf = new Laurus\RRF();
        $this->assertEquals("RRF(k=60)", (string)$rrf);
    }

    public function testWeightedSumRepr(): void
    {
        $ws = new Laurus\WeightedSum();
        $this->assertEquals("WeightedSum(lexical_weight=0.5, vector_weight=0.5)", (string)$ws);
    }

    // ── Analysis pipeline ───────────────────────────────────────────────

    public function testWhitespaceTokenizer(): void
    {
        $tok = new Laurus\WhitespaceTokenizer();
        $tokens = $tok->tokenize("hello world foo");
        $this->assertCount(3, $tokens);
        $this->assertEquals("hello", $tokens[0]->getText());
        $this->assertEquals("world", $tokens[1]->getText());
        $this->assertEquals("foo", $tokens[2]->getText());
    }

    public function testSynonymDictionary(): void
    {
        $dict = new Laurus\SynonymDictionary();
        $dict->addSynonymGroup(["quick", "fast", "speedy"]);
        $this->assertNotNull($dict);
    }

    public function testSynonymGraphFilter(): void
    {
        $dict = new Laurus\SynonymDictionary();
        $dict->addSynonymGroup(["happy", "joyful"]);
        $tok = new Laurus\WhitespaceTokenizer();
        $tokens = $tok->tokenize("I am happy");
        $filter = new Laurus\SynonymGraphFilter($dict);
        $expanded = $filter->apply($tokens);
        $texts = array_map(fn($t) => $t->getText(), $expanded);
        $this->assertContains("happy", $texts);
        $this->assertContains("joyful", $texts);
    }

    // ── SearchResult ────────────────────────────────────────────────────

    public function testSearchResultDocument(): void
    {
        $idx = $this->createIndex();
        $results = $idx->search("title:rust");
        $this->assertCount(1, $results);
        $doc = $results[0]->getDocument();
        $this->assertIsArray($doc);
        $this->assertEquals("Introduction to Rust", $doc["title"]);
    }

    // ── __toString ──────────────────────────────────────────────────────

    public function testTermQueryToString(): void
    {
        $q = new Laurus\TermQuery("title", "hello");
        $this->assertEquals("TermQuery(field='title', term='hello')", (string)$q);
    }

    public function testSearchResultToString(): void
    {
        $idx = $this->createIndex();
        $results = $idx->search("title:rust");
        $str = (string)$results[0];
        $this->assertStringContainsString("SearchResult(", $str);
        $this->assertStringContainsString("doc1", $str);
    }
}
