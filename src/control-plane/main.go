package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

func main() {
	logger, _ := zap.NewProduction()
	defer logger.Sync()

	nc, err := nats.Connect("nats://localhost:4222",
		nats.MaxReconnects(10),
		nats.ReconnectWait(time.Second),
	)
	if err != nil {
		logger.Fatal("nats connection failed", zap.Error(err))
	}
	defer nc.Drain()

	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.Use(gin.Recovery())

	r.Use(cors.New(cors.Config{
		AllowOrigins:     []string{"http://localhost:3000"},
		AllowMethods:     []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowHeaders:     []string{"Origin", "Content-Type", "Accept", "Authorization"},
		ExposeHeaders:    []string{"Content-Length"},
		AllowCredentials: true,
		MaxAge:           12 * time.Hour,
	}))

	api := r.Group("/api/v1")
	{
		api.GET("/health", func(c *gin.Context) {
			c.JSON(http.StatusOK, gin.H{
				"status":    "healthy",
				"service":   "cache-orbit-control-plane",
				"timestamp": time.Now().UnixMilli(),
				"version":   "0.1.0",
			})
		})

		api.GET("/topology", func(c *gin.Context) {
			topology := map[string]interface{}{
				"version": "1.0.0",
				"nodes": []map[string]interface{}{
					{
						"id":         "node-1",
						"address":    "localhost:6379",
						"status":     "healthy",
						"datacenter": "eu-west-1",
						"partitions": []int64{0, 1, 2, 3, 4, 5, 6, 7},
					},
					{
						"id":         "node-2",
						"address":    "localhost:6380",
						"status":     "healthy",
						"datacenter": "us-east-1",
						"partitions": []int64{8, 9, 10, 11, 12, 13, 14, 15},
					},
				},
				"statistics": map[string]interface{}{
					"totalKeys":           145000,
					"hitRate":             0.942,
					"avgLatencyMs":        0.8,
					"p99LatencyMs":        4.2,
					"invalidationsPerSec": 1820,
				},
			}
			c.JSON(http.StatusOK, topology)
		})

		api.POST("/invalidation", func(c *gin.Context) {
			var req struct {
				Key        string `json:"key" binding:"required"`
				Scope      string `json:"scope" binding:"required"`
				ForceFlush bool   `json:"forceFlush"`
			}
			if err := c.ShouldBindJSON(&req); err != nil {
				c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
				return
			}

			_ = nc.Publish("cache.invalidate", []byte(req.Key))
			logger.Info("invalidating key", zap.String("key", req.Key))
			c.JSON(http.StatusOK, gin.H{
				"success": true,
				"key":     req.Key,
			})
		})

		api.POST("/benchmark/start", func(c *gin.Context) {
			var req struct {
				Scenario     string `json:"scenario" binding:"required"`
				RequestCount int    `json:"requestCount"`
				Concurrency  int    `json:"concurrency"`
				WriteRatio   int    `json:"writeRatio"`
			}
			if err := c.ShouldBindJSON(&req); err != nil {
				c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
				return
			}

			benchID := fmt.Sprintf("bench-%d", time.Now().UnixNano())
			go runBenchmark(benchID, req.Scenario, req.RequestCount, req.Concurrency, req.WriteRatio, logger)

			c.JSON(http.StatusOK, gin.H{
				"benchId": benchID,
				"status":  "started",
			})
		})
	}

	srv := &http.Server{
		Addr:    ":8080",
		Handler: r,
	}

	go func() {
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal("server error", zap.Error(err))
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		logger.Error("server shutdown error", zap.Error(err))
	}
}

func runBenchmark(id, scenario string, count, concurrency, writeRatio int, logger *zap.Logger) {
	logger.Info("starting benchmark",
		zap.String("id", id),
		zap.String("scenario", scenario),
		zap.Int("count", count),
		zap.Int("concurrency", concurrency))

	if count == 0 {
		count = 10000
	}
	if concurrency == 0 {
		concurrency = 50
	}
	if writeRatio < 0 || writeRatio > 100 {
		writeRatio = 10
	}

	fmt.Printf("▶ Running benchmark: scenario=%s count=%d concurrency=%d writeRatio=%d\n",
		scenario, count, concurrency, writeRatio)

	start := time.Now()
	completed := 0
	for completed < count {
		ops := concurrency
		if ops > count-completed {
			ops = count - completed
		}
		completed += ops
		time.Sleep(10 * time.Millisecond)
	}

	elapsed := time.Since(start).Seconds()
	opsPerSec := float64(completed) / elapsed

	fmt.Printf("✔ Benchmark %s completed: %d ops in %.2fs (~%.0f ops/s)\n",
		id, completed, elapsed, opsPerSec)
}
