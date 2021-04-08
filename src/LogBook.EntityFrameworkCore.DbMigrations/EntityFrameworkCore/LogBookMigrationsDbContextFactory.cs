using System.IO;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;
using Microsoft.Extensions.Configuration;

namespace LogBook.EntityFrameworkCore
{
    /* This class is needed for EF Core console commands
     * (like Add-Migration and Update-Database commands) */
    public class LogBookMigrationsDbContextFactory : IDesignTimeDbContextFactory<LogBookMigrationsDbContext>
    {
        public LogBookMigrationsDbContext CreateDbContext(string[] args)
        {
            LogBookEfCoreEntityExtensionMappings.Configure();

            var configuration = BuildConfiguration();

            var builder = new DbContextOptionsBuilder<LogBookMigrationsDbContext>()
                .UseSqlServer(configuration.GetConnectionString("Default"));

            return new LogBookMigrationsDbContext(builder.Options);
        }

        private static IConfigurationRoot BuildConfiguration()
        {
            var builder = new ConfigurationBuilder()
                .SetBasePath(Path.Combine(Directory.GetCurrentDirectory(), "../LogBook.DbMigrator/"))
                .AddJsonFile("appsettings.json", optional: false);

            return builder.Build();
        }
    }
}
